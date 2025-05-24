use std::{sync::Arc};

use esi::{
    market::Market, universe::{RegionID, Regions}, ESIClient
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let max_fds: usize;
    #[cfg(unix)]
    {
        rlimit::setrlimit(rlimit::Resource::NOFILE, 1024, 2048).unwrap();
        max_fds = 128;
    }

    #[cfg(windows)]
    {
        rlimit::setmaxstdio(2048).unwrap();
    }

    #[cfg(not(any(unix, windows)))]
    {
        panic!("Unsupported OS!");
    }

    let client = Arc::new(ESIClient::new("market_data_fetcher", std::env::consts::OS, max_fds));
    let regions = Regions::get_all(client.clone()).await.unwrap();
    let forge_orders = Market::fetch_region(RegionID::try_from(10000002).unwrap(), client.clone())
        .await
        .unwrap();

    // need to run a job every 5 minutes to grab new order data
    println!("regions: {:?}", regions.map.len());
    println!("orders: {:?}", forge_orders.items.len());

    Ok(())
}
