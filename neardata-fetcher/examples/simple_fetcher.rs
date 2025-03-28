use fastnear_neardata_fetcher::fetcher;
use fastnear_primitives::block_with_tx_hash::BlockWithTxHashes;
use fastnear_primitives::near_indexer_primitives::types::Finality;
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
    let finality: Finality =
        serde_json::from_str(&format!("{:?}", input("Enter finality", Some("final"))?))
            .expect("Invalid finality");

    let start_block_height = input("Enter start block height (empty - from latest)", Some(""))?;
    let end_block_height = input("Enter end block height (empty - no end)", Some(""))?;
    let num_threads = input("Enter the number of threads", Some("8"))?;
    let auth_bearer_token = input("Enter the auth bearer token (optional)", None)?;

    println!("Starting fetcher");

    let is_running = Arc::new(AtomicBool::new(true));
    let ctrl_c_running = is_running.clone();

    ctrlc::set_handler(move || {
        ctrl_c_running.store(false, Ordering::SeqCst);
        println!("Received Ctrl+C, starting shutdown...");
    })
    .expect("Error setting Ctrl+C handler");

    let mut fetcher_config_builder = fetcher::FetcherConfigBuilder::new()
        .num_threads(num_threads.parse::<u64>().unwrap())
        .chain_id(chain_id)
        .finality(finality);
    if !start_block_height.is_empty() {
        fetcher_config_builder =
            fetcher_config_builder.start_block_height(start_block_height.parse().unwrap());
    }
    if !end_block_height.is_empty() {
        fetcher_config_builder =
            fetcher_config_builder.end_block_height(end_block_height.parse().unwrap());
    }
    if !auth_bearer_token.is_empty() {
        fetcher_config_builder = fetcher_config_builder.auth_bearer_token(auth_bearer_token);
    }
    let fetcher_config = fetcher_config_builder.build();

    let (sender, receiver) = mpsc::channel(100);
    tokio::spawn(fetcher::start_fetcher(fetcher_config, sender, is_running));

    listen_blocks(receiver).await;

    Ok(())
}

async fn listen_blocks(mut stream: mpsc::Receiver<BlockWithTxHashes>) {
    let mut prev_block_hash = None;
    let mut prev_reported_block_height = 0;
    let mut start_time = std::time::Instant::now();
    while let Some(block) = stream.recv().await {
        let block_height = block.block.header.height;
        if start_time.elapsed().as_secs_f64() >= 5.0 {
            println!(
                "Processing block {} ({:.2} blocks per second)",
                block_height,
                if prev_reported_block_height > 0 {
                    (block_height - prev_reported_block_height) as f64
                        / start_time.elapsed().as_secs_f64()
                } else {
                    0.0
                }
            );
            prev_reported_block_height = block_height;
            start_time = std::time::Instant::now();
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
