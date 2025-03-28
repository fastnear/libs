# fastnear-neardata-fetcher

This crate provides a fetcher to retrieve data from neardata.xyz

Handle ctrl-c signal to stop the fetcher.

```rust
pub fn running() -> Arc<AtomicBool> {
    let is_running = Arc::new(AtomicBool::new(true));
    let ctrl_c_running = is_running.clone();

    ctrlc::set_handler(move || {
        ctrl_c_running.store(false, Ordering::SeqCst);
        println!("Received Ctrl+C, starting shutdown...");
    })
        .expect("Error setting Ctrl+C handler");

    is_running
}
```

Configure the fetcher using `FetcherConfigBuilder`, e.g.

```rust
pub fn fetcher_config() -> fetcher::FetcherConfig {
    let mut fetcher_config_builder = fetcher::FetcherConfigBuilder::new()
        .num_threads(num_threads.parse::<u64>().unwrap())
        .chain_id(chain_id)
        .finality(finality);
    if !auth_bearer_token.is_empty() {
        fetcher_config_builder = fetcher_config_builder.auth_bearer_token(auth_bearer_token);
    }
    fetcher_config_builder.build()
}
```

Create a channel, and start a fetcher:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (sender, receiver) = mpsc::channel(100);
    tokio::spawn(fetcher::start_fetcher(fetcher_config(), sender, running()));

    listen_blocks(receiver).await;
}
```

See `examples` for more details.
