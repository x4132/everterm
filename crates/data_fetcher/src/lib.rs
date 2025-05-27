use std::{sync::Arc, time::Duration};

use chrono::Utc;
use dashmap::DashMap;
use esi::{ESIClient, market::Market, universe::RegionID};
use tokio::{sync::{mpsc, Mutex}, time};

pub fn init_io() -> usize {
    #[cfg(unix)]
    {
        rlimit::setrlimit(rlimit::Resource::NOFILE, 1024, 2048).unwrap();
        return 32;
    }

    #[cfg(windows)]
    {
        rlimit::setmaxstdio(2048).unwrap();
        return 32;
    }

    #[cfg(not(any(unix, windows)))]
    {
        panic!("Unsupported OS!");
    }
}

/// This function updates the data for a region whenever it expires.
pub async fn refresh_region_data(region: RegionID, client: Arc<ESIClient>, channel: mpsc::Sender<(Market, RegionID)>) {
    const ERROR_RETRY_DELAY: Duration = Duration::from_secs(15);

    loop {
        let data = Market::fetch_region(region, client.clone()).await;

        match data {
            Ok(data) => {

                time::sleep((data.expires - Utc::now()).to_std().unwrap()).await;
            }
            Err(err) => {
                eprintln!("{:?}", err);

                time::sleep(ERROR_RETRY_DELAY).await;
            }
        }
    }
}

pub async fn update_market_data(book: Arc<Mutex<Market>>, mut rx: mpsc::Receiver<(Market, RegionID)>) {
    let regions: Arc<DashMap<RegionID, Market>> = Arc::new(DashMap::new());
    while let Some((market, region)) = rx.recv().await {
        let regions = regions.clone();
        tokio::spawn(async move {
            let prev = regions.insert(region, market);
        });
    }
}