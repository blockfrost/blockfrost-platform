//! Integration tests for [`bf_common::tcp_mux_tunnel`].
//!
//! We spin up two [`Tunnel`] peers ("Alice" on 127.0.0.1, "Bob" on 127.0.0.2 on
//! Linux, 127.0.0.1 elsewhere)
//! and relay their [`TunnelMsg`]s through an async channel pair that stands in
//! for a WebSocket connection. Bob runs a real TCP echo service; Alice exposes
//! a listener that clients connect to. Data flows:
//!
//! ```text
//! client
//!  │ TCP
//! Alice-listener
//!  │ tunnel
//! Bob (echo service)
//!  │ tunnel
//! Alice
//!  │ TCP
//! client
//! ```
//!
//! This tests the full mux path (Open / Data / Close, base64 encoding,
//! chunking, multiple concurrent streams, cancellation).

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bf_common::tcp_mux_tunnel::{Tunnel, TunnelConfig, TunnelMsg, close_code};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    time::{Duration, timeout},
};
use tokio_util::sync::CancellationToken;

const ALICE_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const BOB_IP: IpAddr = IpAddr::V4(if cfg!(target_os = "linux") {
    Ipv4Addr::new(127, 0, 0, 2)
} else {
    Ipv4Addr::new(127, 0, 0, 1)
});

const TEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Start a TCP echo server that reads and writes back everything. Returns
/// (listener-port, join-handle).
async fn spawn_echo_server(bind_ip: IpAddr) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind(SocketAddr::new(bind_ip, 0))
        .await
        .expect("echo: bind");
    let port = listener.local_addr().unwrap().port();

    let handle = tokio::spawn(async move {
        loop {
            let (mut sock, _) = match listener.accept().await {
                Ok(v) => v,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 8192];
                loop {
                    let n = match sock.read(&mut buf).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    if sock.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
            });
        }
    });
    (port, handle)
}

/// Wire two Tunnel outbound channels together: msgs produced by one peer are
/// fed into the other’s `on_msg`. Returns a join handle that runs until both
/// channels close or the token is cancelled.
fn spawn_relay(
    cancel: CancellationToken,
    alice: Tunnel,
    mut alice_rx: mpsc::Receiver<TunnelMsg>,
    bob: Tunnel,
    mut bob_rx: mpsc::Receiver<TunnelMsg>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                msg = alice_rx.recv() => match msg {
                    Some(m) => { let _ = bob.on_msg(m).await; }
                    None => break,
                },
                msg = bob_rx.recv() => match msg {
                    Some(m) => { let _ = alice.on_msg(m).await; }
                    None => break,
                },
            }
        }
    })
}

/// Build an (Alice, Bob) tunnel pair wired together via a relay task.
/// Alice listens on `alice_listen_port`; Bob exposes `bob_echo_port`.
/// Returns (alice_listen_port actually bound, cancel token, relay handle).
async fn setup_tunnel_pair(
    bob_echo_port: u16,
) -> (u16, CancellationToken, tokio::task::JoinHandle<()>) {
    let cancel = CancellationToken::new();

    // `spawn_listener(0)` lets the OS pick a free port but doesn't return it,
    // so we probe for a free port first and pass it explicitly.
    let probe = TcpListener::bind(SocketAddr::new(ALICE_IP, 0))
        .await
        .expect("probe bind");
    let alice_port = probe.local_addr().unwrap().port();
    drop(probe);

    let alice_cfg = TunnelConfig {
        local_connect_host: ALICE_IP,
        expose_port: 0, // Alice doesn't expose a service
        id_prefix_bit: false,
        ..TunnelConfig::default()
    };
    let bob_cfg = TunnelConfig {
        local_connect_host: BOB_IP,
        expose_port: bob_echo_port,
        id_prefix_bit: true,
        ..TunnelConfig::default()
    };

    let (alice, alice_rx) = Tunnel::new(alice_cfg, cancel.clone());
    let (bob, bob_rx) = Tunnel::new(bob_cfg, cancel.clone());

    let relay = spawn_relay(cancel.clone(), alice.clone(), alice_rx, bob, bob_rx);

    alice
        .spawn_listener(alice_port)
        .await
        .expect("alice: spawn_listener");

    (alice_port, cancel, relay)
}

/// Basic round-trip: one client sends a message through the tunnel and receives
/// the echo back.
#[tokio::test]
async fn single_stream_echo() {
    let (echo_port, _echo_handle) = spawn_echo_server(BOB_IP).await;
    let (alice_port, cancel, _relay) = setup_tunnel_pair(echo_port).await;

    timeout(TEST_TIMEOUT, async {
        let mut client = TcpStream::connect(SocketAddr::new(ALICE_IP, alice_port))
            .await
            .expect("client connect");

        let payload = b"hello through the tunnel!";
        client.write_all(payload).await.expect("client write");

        let mut buf = vec![0u8; payload.len()];
        client.read_exact(&mut buf).await.expect("client read");
        assert_eq!(&buf, payload);

        cancel.cancel();
    })
    .await
    .expect("test timed out");
}

/// Multiple clients sending concurrently. Proves stream mux isolation.
#[tokio::test]
async fn multiple_concurrent_streams() {
    let (echo_port, _echo_handle) = spawn_echo_server(BOB_IP).await;
    let (alice_port, cancel, _relay) = setup_tunnel_pair(echo_port).await;

    timeout(TEST_TIMEOUT, async {
        let mut handles = Vec::new();
        for i in 0u32..8 {
            let addr = SocketAddr::new(ALICE_IP, alice_port);
            handles.push(tokio::spawn(async move {
                let mut client = TcpStream::connect(addr).await.expect("client connect");
                let payload = format!("stream-{i}-payload-{}", "x".repeat(200));
                let payload_bytes = payload.as_bytes();

                client.write_all(payload_bytes).await.expect("write");

                let mut buf = vec![0u8; payload_bytes.len()];
                client.read_exact(&mut buf).await.expect("read");
                assert_eq!(buf, payload_bytes, "stream {i} mismatch");
            }));
        }

        for h in handles {
            h.await.expect("stream task panicked");
        }

        cancel.cancel();
    })
    .await
    .expect("test timed out");
}

/// Payload larger than `read_chunk` (default 8 KiB) to test chunked transfer.
#[tokio::test]
async fn large_payload_chunked() {
    let (echo_port, _echo_handle) = spawn_echo_server(BOB_IP).await;
    let (alice_port, cancel, _relay) = setup_tunnel_pair(echo_port).await;

    timeout(TEST_TIMEOUT, async {
        let mut client = TcpStream::connect(SocketAddr::new(ALICE_IP, alice_port))
            .await
            .expect("client connect");

        // 64 KiB, well above the 8 KiB `read_chunk` default.
        let payload: Vec<u8> = (0..(64 * 1024)).map(|i| (i % 251) as u8).collect();

        client.write_all(&payload).await.expect("write");

        // Read back exactly the same number of bytes (don’t shutdown, as that
        // would race with echoed data still in the tunnel).
        let mut result = vec![0u8; payload.len()];
        client.read_exact(&mut result).await.expect("read");
        assert_eq!(result, payload, "content mismatch");

        cancel.cancel();
    })
    .await
    .expect("test timed out");
}

/// When the peer’s `expose_port` is unreachable, we should get a `Close` back
/// with IO code rather than hanging forever.
#[tokio::test]
async fn unreachable_service_produces_close() {
    let cancel = CancellationToken::new();

    // Pick a port nobody listens on.
    let dead_port = {
        let l = TcpListener::bind(SocketAddr::new(BOB_IP, 0))
            .await
            .expect("probe");
        let p = l.local_addr().unwrap().port();
        drop(l);
        p
    };

    let alice_cfg = TunnelConfig {
        local_connect_host: ALICE_IP,
        expose_port: 0,
        id_prefix_bit: false,
        ..TunnelConfig::default()
    };
    let bob_cfg = TunnelConfig {
        local_connect_host: BOB_IP,
        expose_port: dead_port,
        id_prefix_bit: true,
        ..TunnelConfig::default()
    };

    let (_alice, _alice_rx) = Tunnel::new(alice_cfg, cancel.clone());
    let (bob, mut bob_rx) = Tunnel::new(bob_cfg, cancel.clone());

    timeout(TEST_TIMEOUT, async {
        // Alice sends `Open`, let’s relay it to Bob manually.
        let id = 42u64;
        bob.on_msg(TunnelMsg::Open { id })
            .await
            .expect("on_msg Open");

        // Bob should have produced a `Close` with IO code.
        let msg = bob_rx.recv().await.expect("expected Close from Bob");
        match msg {
            TunnelMsg::Close { id: cid, code, .. } => {
                assert_eq!(cid, id);
                assert_eq!(code, close_code::IO, "expected IO close code");
            },
            other => panic!("expected Close, got {other:?}"),
        }

        cancel.cancel();
    })
    .await
    .expect("test timed out");
}

/// Cancelling the token tears down in-flight connections cleanly.
#[tokio::test]
async fn cancellation_closes_streams() {
    let (echo_port, _echo_handle) = spawn_echo_server(BOB_IP).await;
    let (alice_port, cancel, _relay) = setup_tunnel_pair(echo_port).await;

    timeout(TEST_TIMEOUT, async {
        let mut client = TcpStream::connect(SocketAddr::new(ALICE_IP, alice_port))
            .await
            .expect("client connect");

        // Send something so the tunnel stream is established.
        client.write_all(b"ping").await.expect("write");
        let mut buf = [0u8; 4];
        client.read_exact(&mut buf).await.expect("read ping echo");
        assert_eq!(&buf, b"ping");

        // Now cancel.
        cancel.cancel();

        // Give the runtime a moment to propagate cancellation.
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Subsequent reads/writes should fail or return EOF.
        let mut drain = vec![0u8; 64];
        match client.read(&mut drain).await {
            Ok(0) | Err(_) => {}, // expected
            Ok(n) => panic!("expected EOF or error after cancel, got {n} bytes"),
        }
    })
    .await
    .expect("test timed out");
}
