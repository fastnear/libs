use fastnear_neardata_fetcher::fetcher;
use fastnear_primitives::block_with_tx_hash::BlockWithTxHashes;
use fastnear_primitives::near_primitives::types::BlockHeight;
use fastnear_primitives::types::ChainId;
use std::io;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

pub fn input(query: &str, default: Option<&str>) -> io::Result<String> {
    print!(
        "{}{}: ",
        query,
        default.map_or("".to_string(), |d| format!(" ({})", d))
    );
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_owned();
    if input.is_empty() {
        return Ok(default.unwrap_or_default().to_string());
    }
    Ok(input)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("neardata-fetcher=info")
        .init();

    let chain_id =
        ChainId::try_from(input("Enter chain ID", Some("mainnet"))?).expect("Invalid chain ID");
    let enter_start_block_height = input("Enter start block height", Some("10000000"))?;
    let num_threads = input("Enter the number of threads", Some("8"))?;

    println!("Starting fetcher");

    let is_running = Arc::new(AtomicBool::new(true));
    let ctrl_c_running = is_running.clone();

    ctrlc::set_handler(move || {
        ctrl_c_running.store(false, Ordering::SeqCst);
        println!("Received Ctrl+C, starting shutdown...");
    })
    .expect("Error setting Ctrl+C handler");

    let fetcher_config = fetcher::FetcherConfig {
        num_threads: num_threads.parse::<u64>().unwrap(),
        start_block_height: BlockHeight::try_from(enter_start_block_height.parse::<u64>().unwrap())
            .unwrap(),
        chain_id,
        timeout_duration: None,
        retry_duration: None,
        disable_archive_sync: false,
    };

    let (sender, receiver) = mpsc::channel(100);
    tokio::spawn(fetcher::start_fetcher(
        None,
        fetcher_config,
        sender,
        is_running,
    ));

    listen_blocks(receiver).await;

    Ok(())
}

const REPORT_EVERY: u64 = 1000;

async fn listen_blocks(mut stream: mpsc::Receiver<BlockWithTxHashes>) {
    let mut prev_block_hash = None;
    let mut prev_reported_block_height = 0;
    let mut start_time = std::time::Instant::now();
    while let Some(block) = stream.recv().await {
        let block_height = block.block.header.height;
        if prev_reported_block_height / REPORT_EVERY != block_height / REPORT_EVERY {
            println!(
                "Processing block {} ({:.0} blocks per second)",
                block_height,
                if prev_reported_block_height > 0 {
                    REPORT_EVERY as f64 / start_time.elapsed().as_secs_f64()
                } else {
                    0.0
                }
            );
            start_time = std::time::Instant::now();
            prev_reported_block_height = block_height;
        }

        let block_hash = block.block.header.hash.clone();
        if let Some(prev_block_hash) = prev_block_hash {
            assert_eq!(
                prev_block_hash, block.block.header.prev_hash,
                "Block hashes don't match at block height: {}",
                block.block.header.height
            );
        }
        prev_block_hash = Some(block_hash.clone());
    }
}
