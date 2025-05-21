use esi::{
    ESIClient,
    universe::{Regions},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ESIClient::new("market_data_fetcher", std::env::consts::OS);
    let regions = Regions::fetch_all(client).await.unwrap();
    // let region = Region::get_region(10000020).await.unwrap();
    // let forge_orders = MarketRegion::fetch_orders(region).await.unwrap();

    println!("regions: {:?}", regions.map);

    Ok(())
}
