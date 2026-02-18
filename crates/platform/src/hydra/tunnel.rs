use anyhow::Result;
use bytes::{Bytes, BytesMut};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub enum TunnelEventThere {
    Connect {
        id: u64,
        write_back: mpsc::Sender<TunnelEventBack>,
    },
    Write {
        id: u64,
        chunk: Bytes,
    },
    Disconnect {
        id: u64,
        reason: Option<std::io::Error>,
    },
}

pub enum TunnelEventBack {
    Write { chunk: Bytes },
    Disconnect { reason: Option<std::io::Error> },
}

pub mod connect_here {
    use super::*;
    use std::collections::HashMap;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tokio::task::JoinSet;

    enum ConnCmd {
        Write(Bytes),
        Disconnect(Option<std::io::Error>),
    }

    pub async fn run_tunnel(
        connect_port: u16,
        mut event_rx: mpsc::Receiver<TunnelEventThere>,
        cancel: CancellationToken,
    ) -> Result<()> {
        let mut conns: HashMap<u64, mpsc::Sender<ConnCmd>> = HashMap::new();
        let mut joinset: JoinSet<u64> = JoinSet::new();

        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,

                // Route incoming tunnel events to per-connection tasks.
                ev = event_rx.recv() => {
                    let Some(ev) = ev else { break; };

                    match ev {
                        TunnelEventThere::Connect { id, write_back } => {
                            // If a duplicate id appears, drop the old sender (old task will see recv(None) and exit).
                            conns.remove(&id);

                            let (cmd_tx, cmd_rx) = mpsc::channel::<ConnCmd>(128);
                            conns.insert(id, cmd_tx);

                            let cancel_conn = cancel.clone();
                            joinset.spawn(async move {
                                run_one_connection(connect_port, id, write_back, cmd_rx, cancel_conn).await;
                                id
                            });
                        }

                        TunnelEventThere::Write { id, chunk } => {
                            if let Some(tx) = conns.get(&id) {
                                if tx.send(ConnCmd::Write(chunk)).await.is_err() {
                                    conns.remove(&id);
                                }
                            }
                        }

                        TunnelEventThere::Disconnect { id, reason } => {
                            if let Some(tx) = conns.remove(&id) {
                                // Best-effort; if it fails, the task is already gone.
                                let _ = tx.send(ConnCmd::Disconnect(reason)).await;
                            }
                        }
                    }
                }

                // Reap finished per-connection tasks and drop their routing entry.
                res = joinset.join_next(), if !joinset.is_empty() => {
                    if let Some(Ok(id)) = res {
                        conns.remove(&id);
                    }
                }
            }
        }

        // Stop all remaining connections (dropping the senders makes cmd_rx.recv() return None).
        conns.clear();
        while joinset.join_next().await.is_some() {}

        Ok(())
    }

    async fn run_one_connection(
        connect_port: u16,
        _id: u64,
        write_back: mpsc::Sender<TunnelEventBack>,
        mut cmd_rx: mpsc::Receiver<ConnCmd>,
        cancel: CancellationToken,
    ) {
        let mut sock = match TcpStream::connect(("127.0.0.1", connect_port)).await {
            Ok(s) => s,
            Err(e) => {
                let _ = write_back
                    .send(TunnelEventBack::Disconnect { reason: Some(e) })
                    .await;
                return;
            },
        };

        let mut buf = BytesMut::with_capacity(8 * 1024);
        let cancel_err =
            || std::io::Error::new(std::io::ErrorKind::Interrupted, "tunnel cancelled");

        let mut notify_disconnect = true;
        let reason: Option<std::io::Error> = loop {
            tokio::select! {
                _ = cancel.cancelled() => break Some(cancel_err()),

                rv = async {
                    buf.clear();
                    buf.reserve(8 * 1024);
                    sock.read_buf(&mut buf).await
                } => {
                    match rv {
                        Ok(0) => break None, // clean EOF
                        Ok(_) => {
                            let chunk = buf.split().freeze();
                            if write_back.send(TunnelEventBack::Write { chunk }).await.is_err() {
                                // Other side is gone; no point continuing.
                                notify_disconnect = false;
                                break None;
                            }
                        }
                        Err(e) => break Some(e),
                    }
                }

                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(ConnCmd::Write(chunk)) => {
                            if let Err(e) = sock.write_all(&chunk).await {
                                break Some(e);
                            }
                        }
                        Some(ConnCmd::Disconnect(r)) => {
                            // Peer initiated; don't bother echoing a Disconnect back.
                            notify_disconnect = false;
                            break r;
                        }
                        None => {
                            // Router dropped; exit quietly.
                            notify_disconnect = false;
                            break None;
                        }
                    }
                }
            }
        };

        if notify_disconnect {
            let _ = write_back
                .send(TunnelEventBack::Disconnect { reason })
                .await;
        }

        let _ = sock.shutdown().await;
    }
}

pub mod listen_here {
    use super::*;

    static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

    pub async fn run_tunnel(
        listen_port: u16,
        event_tx_: mpsc::Sender<TunnelEventThere>,
        cancel: CancellationToken,
    ) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind(("127.0.0.1", listen_port)).await?;

        loop {
            let (mut sock, _peer) = tokio::select! {
                _ = cancel.cancelled() => break,
                res = listener.accept() => res?,
            };

            let event_tx = event_tx_.clone();
            let cancel_conn = cancel.clone();

            tokio::spawn(async move {
                let conn_id = NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed);
                let (event_back_tx, mut event_back_rx) = mpsc::channel::<TunnelEventBack>(128);

                // If the tunnel receiver is gone, close the socket/task.
                if event_tx
                    .send(TunnelEventThere::Connect {
                        id: conn_id,
                        write_back: event_back_tx,
                    })
                    .await
                    .is_err()
                {
                    let _ = sock.shutdown().await;
                    return;
                }

                let mut buf = BytesMut::with_capacity(8 * 1024);
                let cancel_err =
                    || std::io::Error::new(std::io::ErrorKind::Interrupted, "tunnel cancelled");

                let reason: Option<std::io::Error> = loop {
                    tokio::select! {
                        _ = cancel_conn.cancelled() => break Some(cancel_err()),

                        rv = async {
                            buf.clear();
                            buf.reserve(8 * 1024);
                            sock.read_buf(&mut buf).await
                        } => {
                            match rv {
                                Ok(0) => break None, // clean EOF
                                Ok(_) => {
                                    let chunk = buf.split().freeze(); // no copy
                                    if event_tx.send(TunnelEventThere::Write { id: conn_id, chunk }).await.is_err() {
                                        let _ = sock.shutdown().await;
                                        return;
                                    }
                                }
                                Err(e) => break Some(e),
                            }
                        }

                        event_back = event_back_rx.recv() => {
                            match event_back {
                                Some(TunnelEventBack::Write { chunk }) => {
                                    if let Err(e) = sock.write_all(&chunk).await {
                                        break Some(e);
                                    }
                                }
                                Some(TunnelEventBack::Disconnect { reason }) => break reason,
                                None => break None, // all back-senders dropped
                            }
                        }
                    }
                };

                // Best-effort; if it fails, the tunnel is already gone.
                let _ = event_tx
                    .send(TunnelEventThere::Disconnect {
                        id: conn_id,
                        reason,
                    })
                    .await;

                let _ = sock.shutdown().await;
            });
        }

        Ok(())
    }
}
