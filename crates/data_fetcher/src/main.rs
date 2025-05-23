use std::sync::Arc;

use esi::{
    market::Market, universe::{RegionID, Regions}, ESIClient
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(ESIClient::new("market_data_fetcher", std::env::consts::OS));
    let regions = Regions::get_all(client.clone()).await.unwrap();
    let forge_orders = Market::fetch_region(RegionID::try_from(10000002).unwrap(), client.clone())
        .await
        .unwrap();

    // need to run a job every 5 minutes to grab new order data
    println!("regions: {:?}", regions.map.len());
    println!("orders: {:?}", forge_orders.items.len());

    Ok(())
}
