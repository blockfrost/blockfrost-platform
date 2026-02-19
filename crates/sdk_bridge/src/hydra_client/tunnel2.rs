use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{Mutex, mpsc},
};
use tokio_util::sync::CancellationToken;

/// JSON-serializable tunnel messages (base64 for buffers).
///
/// Plug into a WebSocket protocol as e.g. `WsProto::HydraTunnel(TunnelMsg)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum TunnelMsg {
    /// Ask peer to open its *configured* local service port for stream `id`.
    Open { id: u64 },

    /// Bytes for connection `id` encoded as base64 string.
    Data { id: u64, b64: String },

    /// Close stream `id`. `code` is small+stable, `msg` is optional.
    Close {
        id: u64,
        code: u8,
        msg: Option<String>,
    },
}

pub mod close_code {
    pub const CLEAN: u8 = 0;
    pub const IO: u8 = 1;
    pub const CANCELLED: u8 = 2;
    pub const PROTOCOL: u8 = 3;
}

/// Tunnel config.
#[derive(Debug, Clone)]
pub struct TunnelConfig {
    /// Host used for local TCP connects when peer sends Open.
    pub local_connect_host: IpAddr,

    /// The *only* local port that the peer is allowed to connect to (via Open).
    pub expose_port: u16,

    /// If true, set bit 63 in all locally-allocated IDs.
    /// Set opposite values on the two peers to avoid ID collisions.
    pub id_prefix_bit: bool,

    /// Outbound TunnelMsg buffer (what the WebSocket event loop drains).
    pub outbound_capacity: usize,

    /// Per-connection command channel capacity.
    pub per_conn_cmd_capacity: usize,

    /// Max bytes per TCP read.
    pub read_chunk: usize,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            local_connect_host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            expose_port: 0,
            id_prefix_bit: false,
            outbound_capacity: 256,
            per_conn_cmd_capacity: 128,
            read_chunk: 8 * 1024,
        }
    }
}

enum ConnCmd {
    Write(Bytes),
    /// Close local TCP. If notify_peer=false, don’t emit a Close back.
    Close {
        notify_peer: bool,
    },
}

struct Inner {
    cfg: TunnelConfig,
    cancel: CancellationToken,
    out_tx: mpsc::Sender<TunnelMsg>,
    conns: Mutex<HashMap<u64, mpsc::Sender<ConnCmd>>>,
    next_id: AtomicU64,
}

/// Cloneable handle kept in a WebSocket connection state/event-loop.
#[derive(Clone)]
pub struct Tunnel {
    inner: Arc<Inner>,
}

impl Tunnel {
    /// Create tunnel + outbound receiver (to drain in the WebSocket event loop).
    pub fn new(cfg: TunnelConfig, cancel: CancellationToken) -> (Self, mpsc::Receiver<TunnelMsg>) {
        let (out_tx, out_rx) = mpsc::channel(cfg.outbound_capacity);

        let prefix = if cfg.id_prefix_bit { 1u64 << 63 } else { 0 };
        let next_id = AtomicU64::new(1 | prefix);

        let inner = Arc::new(Inner {
            cfg,
            cancel,
            out_tx,
            conns: Mutex::new(HashMap::new()),
            next_id,
        });

        (Self { inner }, out_rx)
    }

    pub fn cancel(&self) {
        self.inner.cancel.cancel();
    }

    /// Call this from the WebSocket event loop when it receives a tunnel message.
    pub async fn on_msg(&self, msg: TunnelMsg) -> Result<()> {
        match msg {
            TunnelMsg::Open { id } => {
                // Always connect to the single configured local port.
                let addr = SocketAddr::new(
                    self.inner.cfg.local_connect_host,
                    self.inner.cfg.expose_port,
                );
                match TcpStream::connect(addr).await {
                    Ok(sock) => self.attach_stream_with_id(id, sock).await?,
                    Err(e) => {
                        let _ = self
                            .inner
                            .out_tx
                            .send(TunnelMsg::Close {
                                id,
                                code: close_code::IO,
                                msg: Some(e.to_string()),
                            })
                            .await;
                    },
                }
            },

            TunnelMsg::Data { id, b64 } => {
                let bytes = match B64.decode(b64.as_bytes()) {
                    Ok(v) => Bytes::from(v),
                    Err(_) => {
                        let _ = self
                            .inner
                            .out_tx
                            .send(TunnelMsg::Close {
                                id,
                                code: close_code::PROTOCOL,
                                msg: Some("invalid base64".into()),
                            })
                            .await;
                        return Ok(());
                    },
                };

                let tx = { self.inner.conns.lock().await.get(&id).cloned() };
                if let Some(tx) = tx {
                    let _ = tx.send(ConnCmd::Write(bytes)).await;
                }
            },

            TunnelMsg::Close { id, .. } => {
                let tx = { self.inner.conns.lock().await.remove(&id) };
                if let Some(tx) = tx {
                    let _ = tx.send(ConnCmd::Close { notify_peer: false }).await;
                }
            },
        }

        Ok(())
    }

    /// Spawn a TCP listener on *this* side. Each accepted local TCP connection becomes
    /// a tunneled stream to the peer’s configured `expose_port`.
    pub async fn spawn_listener(&self, listen_port: u16) -> Result<()> {
        let listener = TcpListener::bind((self.inner.cfg.local_connect_host, listen_port)).await?;
        let this = self.clone();

        tokio::spawn(async move {
            loop {
                let (mut sock, _) = tokio::select! {
                    _ = this.inner.cancel.cancelled() => break,
                    res = listener.accept() => match res { Ok(v) => v, Err(_) => break }
                };

                let id = this.alloc_local_id();

                // Ask peer to open its configured port.
                if this
                    .inner
                    .out_tx
                    .send(TunnelMsg::Open { id })
                    .await
                    .is_err()
                {
                    let _ = sock.shutdown().await;
                    break;
                }

                // Attach local accepted socket.
                if this.attach_stream_with_id(id, sock).await.is_err() {
                    let _ = this
                        .inner
                        .out_tx
                        .send(TunnelMsg::Close {
                            id,
                            code: close_code::PROTOCOL,
                            msg: Some("attach failed".into()),
                        })
                        .await;
                }
            }
        });

        Ok(())
    }

    /// If you already accepted a TcpStream elsewhere and want to tunnel it:
    /// sends Open (no port) and returns the allocated id.
    ///
    /// *Warning*: it’s a little controversial, but trivial to add. Probably
    /// shouldn’t be used.
    #[allow(dead_code)]
    pub async fn attach_stream(&self, sock: TcpStream) -> Result<u64> {
        let id = self.alloc_local_id();
        self.inner.out_tx.send(TunnelMsg::Open { id }).await?;
        self.attach_stream_with_id(id, sock).await?;
        Ok(id)
    }

    fn alloc_local_id(&self) -> u64 {
        let prefix = if self.inner.cfg.id_prefix_bit {
            1u64 << 63
        } else {
            0
        };
        let base = self.inner.next_id.fetch_add(1, Ordering::Relaxed) & !(1u64 << 63);
        base | prefix
    }

    async fn attach_stream_with_id(&self, id: u64, sock: TcpStream) -> Result<()> {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<ConnCmd>(self.inner.cfg.per_conn_cmd_capacity);

        // Insert route, replacing duplicates (and closing old).
        {
            let mut m = self.inner.conns.lock().await;
            if let Some(old) = m.insert(id, cmd_tx) {
                let _ = old.send(ConnCmd::Close { notify_peer: false }).await;
            }
        }

        let out_tx = self.inner.out_tx.clone();
        let cancel = self.inner.cancel.clone();
        let cfg = self.inner.cfg.clone();
        let inner = Arc::clone(&self.inner);

        tokio::spawn(async move {
            let mut sock = sock;
            let mut buf = BytesMut::with_capacity(cfg.read_chunk);
            let mut notify_peer_close = true;

            let close_reason: Option<(u8, Option<String>)> = loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break Some((close_code::CANCELLED, Some("cancelled".into())));
                    }

                    // TCP -> WS (encode bytes as base64)
                    rv = async {
                        buf.clear();
                        buf.reserve(cfg.read_chunk);
                        sock.read_buf(&mut buf).await
                    } => {
                        match rv {
                            Ok(0) => break Some((close_code::CLEAN, None)), // EOF
                            Ok(_) => {
                                let chunk = buf.split().freeze();
                                let b64 = B64.encode(&chunk);
                                if out_tx.send(TunnelMsg::Data { id, b64 }).await.is_err() {
                                    notify_peer_close = false;
                                    break None;
                                }
                            }
                            Err(e) => break Some((close_code::IO, Some(e.to_string()))),
                        }
                    }

                    // WS -> TCP (decode already done in on_msg)
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Some(ConnCmd::Write(chunk)) => {
                                if let Err(e) = sock.write_all(&chunk).await {
                                    break Some((close_code::IO, Some(e.to_string())));
                                }
                            }
                            Some(ConnCmd::Close { notify_peer }) => {
                                notify_peer_close = notify_peer;
                                break Some((close_code::CLEAN, None));
                            }
                            None => {
                                notify_peer_close = false;
                                break None;
                            }
                        }
                    }
                }
            };

            // Remove route
            let _ = inner.conns.lock().await.remove(&id);

            if notify_peer_close {
                let (code, msg) =
                    close_reason.unwrap_or((close_code::CANCELLED, Some("closed".into())));
                let _ = out_tx.send(TunnelMsg::Close { id, code, msg }).await;
            }

            let _ = sock.shutdown().await;
        });

        Ok(())
    }
}
