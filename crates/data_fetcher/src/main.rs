use std::sync::Arc;

use data_fetcher::init_io;
use esi::{
    ESIClient,
    market::Market,
    universe::{Regions},
};
use tokio::sync::{Mutex, mpsc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let max_fds: usize = init_io();

    let client = Arc::new(ESIClient::new(
        "market_data_fetcher",
        std::env::consts::OS,
        max_fds,
    ));
    let regions = Regions::get_all(client.clone()).await.unwrap();

    // set up orderbook fetching
    let book = Arc::new(Mutex::new(Market::new()));
    {
        let (tx, rx) = mpsc::channel(4);
        for region in regions.map.clone().iter() {
            tokio::spawn(data_fetcher::refresh_region_data(
                region.id,
                client.clone(),
                tx.clone(),
            ));
        }

        // orderbook reassembler
        tokio::spawn(data_fetcher::update_market_data(book.clone(), rx));
    }

    return Ok(());
}
