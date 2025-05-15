use universe::Regions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let regions = Regions::fetch().await?;

    Ok(())
}
