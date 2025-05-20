use esi::{market::MarketRegion, universe::Region, ESIClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ESIClient::new("market_data_fetcher", std::env::consts::OS);
    let region = Region::get_region(10000020).await.unwrap();
    let forge_orders = MarketRegion::fetch_orders(region).await.unwrap();

    println!("orders: {}", forge_orders.orders.len());

    Ok(())
}
