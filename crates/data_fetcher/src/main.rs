use std::sync::Arc;

use esi::{
    ESIClient,
    market::Market,
    universe::{RegionID, Regions},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let max_fds: usize;
    #[cfg(unix)]
    {
        rlimit::setrlimit(rlimit::Resource::NOFILE, 1024, 2048).unwrap();
        max_fds = 32;
    }

    #[cfg(windows)]
    {
        rlimit::setmaxstdio(2048).unwrap();
    }

    #[cfg(not(any(unix, windows)))]
    {
        panic!("Unsupported OS!");
    }

    let client = Arc::new(ESIClient::new(
        "market_data_fetcher",
        std::env::consts::OS,
        max_fds,
    ));
    let regions = Regions::get_all(client.clone()).await.unwrap();

    let mut orders;

    loop {
        orders = fetch_all_orders(&regions, client.clone()).await;

        // TODO: push order map into redis or something

        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
}

async fn fetch_all_orders(regions: &Regions, client: Arc<ESIClient>) -> Market {
    Market::fetch_regions(
        regions.map.iter().map(|i| i.key().to_owned()).collect(),
        client,
    )
    .await
    .unwrap()
}
