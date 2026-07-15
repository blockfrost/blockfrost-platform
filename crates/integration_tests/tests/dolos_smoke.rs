use blockfrost::{BlockfrostAPI, Pagination};
use integration_tests::{
    dolos_endpoint, get_blockfrost_client, get_platform_client, initialize_logging,
    platform::{build_app_with_data_node, spawn_app},
};
use pretty_assertions::assert_eq;

async fn spawn_platform_with_dolos() -> BlockfrostAPI {
    let (app, _, _, _, _) = build_app_with_data_node(dolos_endpoint())
        .await
        .expect("Failed to build the application");

    get_platform_client(&spawn_app(app).await)
}

/// Hash of an already-settled block, so no compared field is racing the chain
/// tip.
async fn anchor_block_hash(platform: &BlockfrostAPI) -> String {
    platform
        .blocks_latest()
        .await
        .expect("Request to /blocks/latest failed")
        .previous_block
        .expect("Latest block has no previous block")
}

// Test: `/blocks/latest` served from the data node (Dolos) has the same
// response as the Blockfrost API
#[tokio::test]
#[ntest::timeout(120_000)]
async fn test_data_node_blocks_latest_matches_blockfrost() {
    initialize_logging();

    let platform = spawn_platform_with_dolos().await;

    // Local (Platform, backed by Dolos)
    let mut local_block = platform
        .blocks_latest()
        .await
        .expect("Request to /blocks/latest failed");

    // Blockfrost API: fetch the same block by hash — comparing latest against
    // latest would race the chain tip
    let mut blockfrost_block = get_blockfrost_client()
        .blocks_by_id(&local_block.hash)
        .await
        .expect("Blockfrost request failed");

    // Both advance with the chain, so they can differ between the two fetches
    local_block.confirmations = 0;
    blockfrost_block.confirmations = 0;
    local_block.next_block = None;
    blockfrost_block.next_block = None;

    assert_eq!(local_block, blockfrost_block);
}

// Test: `/blocks/{hash}` served from the data node (Dolos) has the same
// response as the Blockfrost API
#[tokio::test]
#[ntest::timeout(120_000)]
async fn test_data_node_blocks_by_id_matches_blockfrost() {
    initialize_logging();

    let platform = spawn_platform_with_dolos().await;
    let anchor = anchor_block_hash(&platform).await;

    let mut local_block = platform
        .blocks_by_id(&anchor)
        .await
        .expect("Request to /blocks/{hash} failed");

    let mut blockfrost_block = get_blockfrost_client()
        .blocks_by_id(&anchor)
        .await
        .expect("Blockfrost request failed");

    // Advances with the chain, so it can differ between the two fetches
    local_block.confirmations = 0;
    blockfrost_block.confirmations = 0;

    assert_eq!(local_block, blockfrost_block);
}

// Test: `/blocks/{hash}/txs` served from the data node (Dolos) has the same
// response as the Blockfrost API
#[tokio::test]
#[ntest::timeout(120_000)]
async fn test_data_node_blocks_txs_match_blockfrost() {
    initialize_logging();

    let platform = spawn_platform_with_dolos().await;
    let anchor = anchor_block_hash(&platform).await;

    let local_txs = platform
        .blocks_txs(&anchor, Pagination::default())
        .await
        .expect("Request to /blocks/{hash}/txs failed");

    let blockfrost_txs = get_blockfrost_client()
        .blocks_txs(&anchor, Pagination::default())
        .await
        .expect("Blockfrost request failed");

    assert_eq!(local_txs, blockfrost_txs);
}
