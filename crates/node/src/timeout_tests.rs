use crate::{connection::NodeClient, monitoring::node_monitor::NodeMonitor, pool::NodePool};
use bf_common::errors::BlockfrostError;
use pallas_codec::utils::AnyCbor;
use pallas_network::{facades::NodeServer, miniprotocols::localstate::ClientQueryRequest};
use tokio::{
    io::AsyncReadExt,
    net::UnixListener,
    sync::oneshot,
    time::{Duration, timeout},
};

const MAGIC: u64 = 2;
const TEST_TIMEOUT: Duration = Duration::from_millis(100);

fn listener() -> (tempfile::TempDir, UnixListener, String) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("node.socket");
    let listener = UnixListener::bind(&path).unwrap();
    (dir, listener, path.to_str().unwrap().to_owned())
}

async fn assert_closed(server: &mut NodeServer) {
    // No chain-sync messages are sent in these tests. An error on this idle
    // channel establishes that the transport closed rather than just timing out.
    assert!(server.chainsync().recv_while_idle().await.is_err());
}

#[tokio::test]
async fn acquisition_timeout_closes_connection() {
    let (_dir, listener, path) = listener();
    let peer = tokio::spawn(async move {
        let mut server = NodeServer::accept(&listener, MAGIC).await.unwrap();
        assert!(
            server
                .statequery()
                .recv_while_idle()
                .await
                .unwrap()
                .is_some()
        );
        // Withhold Acquired while keeping the transport open.
        assert_closed(&mut server).await;
        server.abort().await;
    });
    let mut node = NodeClient::connect(&path, MAGIC, 0).await.unwrap();
    let err = timeout(
        Duration::from_secs(2),
        node.with_statequery_timeout(|_| Box::pin(async { Ok(()) }), TEST_TIMEOUT),
    )
    .await
    .expect("acquisition escaped the deadline")
    .unwrap_err();
    assert_eq!(err.status_code, 408);
    assert!(node.unrecoverable_error_happened);
    assert!(node.client.is_none());
    assert!(node.ping().await.is_err());
    timeout(Duration::from_secs(2), peer)
        .await
        .expect("peer did not observe connection disposal")
        .unwrap();
}

#[tokio::test]
async fn query_timeout_closes_connection_without_releasing() {
    let (_dir, listener, path) = listener();
    let peer = tokio::spawn(async move {
        let mut server = NodeServer::accept(&listener, MAGIC).await.unwrap();
        server.statequery().recv_while_idle().await.unwrap();
        server.statequery().send_acquired().await.unwrap();
        assert!(matches!(
            server.statequery().recv_while_acquired().await.unwrap(),
            ClientQueryRequest::Query(_)
        ));
        // The peer still has agency. Release is not legal here.
        assert_closed(&mut server).await;
        server.abort().await;
    });
    let mut node = NodeClient::connect(&path, MAGIC, 0).await.unwrap();
    let err = node
        .with_statequery_timeout(
            |client| {
                Box::pin(async move {
                    client.send_query(AnyCbor::from_encode(0u8)).await?;
                    client.recv_while_querying().await?;
                    Ok(())
                })
            },
            TEST_TIMEOUT,
        )
        .await
        .unwrap_err();
    assert_eq!(err.status_code, 408);
    assert!(node.client.is_none());
    timeout(Duration::from_secs(2), peer)
        .await
        .expect("peer did not observe connection disposal")
        .unwrap();
}

#[tokio::test]
async fn action_error_releases_connection_and_allows_reuse() {
    let (_dir, listener, path) = listener();
    let (released_tx, released_rx) = oneshot::channel();
    let peer = tokio::spawn(async move {
        let mut server = NodeServer::accept(&listener, MAGIC).await.unwrap();
        for _ in 0..2 {
            server.statequery().recv_while_idle().await.unwrap();
            server.statequery().send_acquired().await.unwrap();
            assert!(matches!(
                server.statequery().recv_while_acquired().await.unwrap(),
                ClientQueryRequest::Release
            ));
        }
        released_tx.send(()).unwrap();
        assert_closed(&mut server).await;
        server.abort().await;
    });
    let mut node = NodeClient::connect(&path, MAGIC, 0).await.unwrap();
    let err = node
        .with_statequery_timeout::<(), _>(
            |_| {
                Box::pin(async {
                    Err(BlockfrostError::custom_400("test action failed".to_owned()))
                })
            },
            Duration::from_secs(2),
        )
        .await
        .unwrap_err();
    assert_eq!(err.status_code, 400);
    assert!(!node.unrecoverable_error_happened);
    node.ping().await.unwrap();
    released_rx.await.unwrap();
    node.close().await;
    timeout(Duration::from_secs(2), peer)
        .await
        .expect("peer did not observe connection disposal")
        .unwrap();
}

#[tokio::test]
async fn checkout_timeout_updates_health_and_discards_stalled_recycle() {
    let (_dir, listener, path) = listener();
    let (acquired_tx, acquired_rx) = oneshot::channel();
    let peer = tokio::spawn(async move {
        let mut server = NodeServer::accept(&listener, MAGIC).await.unwrap();
        server.statequery().recv_while_idle().await.unwrap();
        acquired_tx.send(()).unwrap();
        assert_closed(&mut server).await;
        server.abort().await;
    });
    let pool = NodePool::new(MAGIC, path, 1).unwrap();
    drop(pool.get().await.unwrap());
    let monitor = NodeMonitor::new();
    let errors = monitor.errors();
    assert!(errors.lock().await.is_empty());
    let update = tokio::spawn(async move {
        monitor.update(&pool).await;
        assert!(monitor.node_info().lock().await.is_none());
    });
    acquired_rx.await.unwrap();
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(31)).await;
    tokio::time::resume();
    timeout(Duration::from_secs(2), update)
        .await
        .unwrap()
        .unwrap();
    assert!(!errors.lock().await.is_empty());
    timeout(Duration::from_secs(2), peer)
        .await
        .expect("peer did not observe connection disposal")
        .unwrap();
}

#[tokio::test]
async fn checkout_timeout_bounds_wait_for_capacity() {
    let (_dir, listener, path) = listener();
    let peer = tokio::spawn(async move {
        let mut server = NodeServer::accept(&listener, MAGIC).await.unwrap();
        assert_closed(&mut server).await;
        server.abort().await;
    });
    let pool = NodePool::new(MAGIC, path, 1).unwrap();
    let held = pool.get().await.unwrap();
    tokio::time::pause();
    assert!(pool.get().await.is_err());
    tokio::time::resume();
    // The occupied connection is still usable and was not discarded by the waiter.
    assert!(held.client.is_some());
    drop(held);
    drop(pool);
    timeout(Duration::from_secs(2), peer)
        .await
        .expect("peer did not observe connection disposal")
        .unwrap();
}

#[tokio::test]
async fn checkout_timeout_closes_stalled_handshake() {
    let (_dir, listener, path) = listener();
    let (handshake_tx, handshake_rx) = oneshot::channel();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = [0; 1024];
        assert!(socket.read(&mut bytes).await.unwrap() > 0);
        handshake_tx.send(()).unwrap();
        let mut remaining = Vec::new();
        socket.read_to_end(&mut remaining).await.unwrap();
    });
    let pool = NodePool::new(MAGIC, path, 1).unwrap();
    let checkout = tokio::spawn(async move {
        assert!(pool.get().await.is_err());
    });
    handshake_rx.await.unwrap();
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(31)).await;
    tokio::time::resume();
    timeout(Duration::from_secs(2), checkout)
        .await
        .unwrap()
        .unwrap();
    timeout(Duration::from_secs(2), peer)
        .await
        .expect("peer did not observe connection disposal")
        .unwrap();
}
