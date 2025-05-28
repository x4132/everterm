use std::sync::Arc;

use esi::{market::Market, universe::{Regions}, ESIClient};
use tokio::sync::{Mutex, mpsc, broadcast};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up file descriptor limits in the main thread
    let max_fds: usize = {
        #[cfg(unix)]
        {
            rlimit::setrlimit(rlimit::Resource::NOFILE, 2048, 2048).unwrap();
            32
        }
        #[cfg(windows)]
        {
            rlimit::setmaxstdio(2048).unwrap();
            32
        }

        #[cfg(not(any(unix, windows)))]
        {
            panic!("Unsupported OS!");
        }
    };

    let client = Arc::new(ESIClient::new(
        "market_data_fetcher",
        std::env::consts::OS,
        max_fds,
    ));
    let regions = Regions::get_all(client.clone()).await.unwrap();

    // Set up broadcast channel for region refresh events
    let (region_upd_tx, _region_upd_rx) = broadcast::channel(128);

    // set up orderbook fetching
    let book = Arc::new(Mutex::new(Market::new()));
    {
        let (tx, rx) = mpsc::channel(128);
        for region in regions.map.clone().iter() {
            tokio::spawn(data_fetcher::refresh_region_data(
                region.clone(),
                client.clone(),
                tx.clone(),
                region_upd_tx.clone(),
            ));
        }

        // orderbook reassembler
        tokio::spawn(data_fetcher::update_market_data(book.clone(), rx));
    }

    loop {}
}
