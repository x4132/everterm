use market::MarketRegion;
use universe::Region;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let regions = Regions::fetch_all().await?;

    // println!("len: {}", regions.0.len());

    let region = Region::get_region(10000020).await.unwrap();
    let forge_orders = MarketRegion::fetch_orders(region).await.unwrap();

    println!("orders: {}", forge_orders.orders.len());

    Ok(())
}
