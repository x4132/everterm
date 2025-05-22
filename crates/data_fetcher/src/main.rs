use esi::{
    universe::{Regions, Systems}, ESIClient
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ESIClient::new("market_data_fetcher", std::env::consts::OS);
    // let regions = Regions::get_all(client).await.unwrap();
    let systems = Systems::get_all(client).await.unwrap();
    // let forge_orders = MarketRegion::fetch_orders(region).await.unwrap();

    // println!("regions: {:?}", regions.map);
    println!("systems: {:?}", systems.map);

    Ok(())
}
