use fastnear_neardata_fetcher::fetcher;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_fetch_50_blocks_with_hash_chain() {
    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_clone = is_running.clone();

    let fetcher_config = fetcher::FetcherConfigBuilder::new().build();

    let (sender, mut receiver) = mpsc::channel(100);
    tokio::spawn(fetcher::start_fetcher(
        fetcher_config,
        sender,
        is_running_clone,
    ));

    let mut prev_block_hash = None;
    let mut count = 0u64;

    while let Some(block) = receiver.recv().await {
        let block_height = block.block.header.height;
        let block_hash = block.block.header.hash.clone();

        if let Some(expected_prev_hash) = &prev_block_hash {
            assert_eq!(
                expected_prev_hash, &block.block.header.prev_hash,
                "Block hash chain broken at height {}",
                block_height
            );
        }

        prev_block_hash = Some(block_hash);
        count += 1;

        if count >= 50 {
            break;
        }
    }

    is_running.store(false, Ordering::SeqCst);

    assert_eq!(count, 50, "Expected 50 blocks, got {}", count);
}
