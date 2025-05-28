use std::{sync::Arc};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use data_fetcher::{get_refresh_intervals, server::data_server};
use esi::{
    ESIClient,
    market::Market,
    universe::{Regions},
};
use tokio::sync::{Mutex, broadcast, mpsc};

// so much DI smh

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
    let (region_upd_tx, region_upd_rx) = broadcast::channel(128);

    // set up orderbook fetching
    let market_books = Arc::new(Mutex::new(Market::new()));
    {
        let (tx, rx) = mpsc::channel(128);
        for region in regions.region_map.clone().iter() {
            tokio::spawn(data_fetcher::refresh_region_data(
                region.clone(),
                client.clone(),
                tx.clone(),
                region_upd_tx.clone(),
            ));
        }

        // orderbook reassembler
        tokio::spawn(data_fetcher::update_market_data(market_books.clone(), rx));
    }

    // Handle interval refresh state
    let refresh_intervals: Arc<DashMap<u32, Option<DateTime<Utc>>>> =
        Arc::new(regions.region_map.iter().map(|kv| (kv.id.get(), None)).collect());
    tokio::spawn(get_refresh_intervals(refresh_intervals.clone(), region_upd_rx));


    data_server(refresh_intervals, market_books).await.unwrap();

    Ok(())
}
