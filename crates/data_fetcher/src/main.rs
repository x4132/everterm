use universe::Regions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let regions = Regions::fetch_all().await?;

    println!("len: {}", regions.0.len());

    Ok(())
}
