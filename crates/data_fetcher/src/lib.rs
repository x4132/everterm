use std::{sync::Arc, time::Duration};

use chrono::{DateTime, TimeDelta, Utc};
use dashmap::DashMap;
use esi::{
    ESIClient,
    market::{Market, OrderBook},
    universe::{Region, RegionID},
};
use tokio::{
    sync::{Mutex, mpsc, broadcast},
    time,
};

/// Message broadcast when a region is refreshed
#[derive(Debug, Clone)]
pub struct RegionRefreshEvent {
    pub region_id: RegionID,
    pub expires: DateTime<Utc>,
}

/// This function updates the data for a region whenever it expires.
pub async fn refresh_region_data(
    region: Region,
    client: Arc<ESIClient>,
    channel: mpsc::Sender<(Market, Region)>,
    broadcast_tx: broadcast::Sender<RegionRefreshEvent>,
) {
    const ERROR_RETRY_DELAY: Duration = Duration::from_secs(15);

    loop {
        let data = Market::fetch_region(&region, client.clone()).await;

        match data {
            Ok(data) => {
                // Extract the expiry time before sending the data
                let expiry_time = data.expires;


                let sleep_dur = (expiry_time - Utc::now() + TimeDelta::new(1, 0).unwrap())
                    .to_std()
                    .unwrap_or(std::time::Duration::from_secs(30));

                println!(
                    "Region {} sleeping for {} secs",
                    region.name.clone(),
                    sleep_dur.as_secs()
                );

                // Send the market data through the channel for processing
                if let Err(_) = channel.send((data, region.clone())).await {
                    eprintln!("Failed to send market data for region {}", region.name);
                    break; // Exit if the receiver is dropped
                }

                // Broadcast the region refresh event
                let refresh_event = RegionRefreshEvent {
                    region_id: region.id,
                    expires: expiry_time,
                };

                if let Err(_) = broadcast_tx.send(refresh_event) {
                    // This is not critical - broadcast may not have any receivers
                    // We don't break here as this is just informational
                }

                time::sleep(sleep_dur).await;
            }
            Err(err) => {
                eprintln!("{:?}", err);

                time::sleep(ERROR_RETRY_DELAY).await;
            }
        }
    }
}

pub async fn update_market_data(
    book: Arc<Mutex<Market>>,
    mut rx: mpsc::Receiver<(Market, Region)>,
) {
    let regions: Arc<DashMap<Region, Market>> = Arc::new(DashMap::new());

    while let Some((new_market, region)) = rx.recv().await {
        let regions = regions.clone();
        let book = book.clone();

        tokio::spawn(async move {
            // Store timestamps from the new market
            let new_last_modified = new_market.last_modified;
            let new_expires = new_market.expires;

            println!("Processing market update for region {}", region.name);

            // Calculate the diff between previous and new market data
            let diff = match regions.get(&region) {
                Some(prev_market_ref) => {
                    println!("Computing delta for region {} (update)", region.name);
                    prev_market_ref.delta(&new_market)
                }
                None => {
                    // First time seeing this region - everything is "new"
                    let empty_market = Market::new();
                    println!("Computing delta for region {} (first time)", region.name);
                    empty_market.delta(&new_market)
                }
            };

            // Apply the diff to the global market book
            let mut global_book = book.lock().await;

            // Process removed orders
            let mut removed_ordercount = 0;
            for (item_type, removed_order_ids) in diff.removed {
                if let Some(mut order_book) = global_book.items.get_mut(&item_type) {
                    for order_id in removed_order_ids {
                        removed_ordercount += 1;
                        order_book.orders.remove(&order_id);
                    }
                }
            }

            // Process new orders
            let mut new_ordercount = 0;
            for (item_type, new_orders) in diff.new {
                // Ensure the orderbook exists for this item type
                if !global_book.items.contains_key(&item_type) {
                    global_book
                        .items
                        .insert(item_type, OrderBook::new(item_type));
                }

                if let Some(mut order_book) = global_book.items.get_mut(&item_type) {
                    for order in new_orders {
                        order_book.orders.insert(order.id, order);
                        new_ordercount += 1;
                    }
                }
            }

            // Process modified orders
            let mut modified_ordercount = 0;
            for (item_type, modified_orders) in diff.modified {
                // Ensure the orderbook exists for this item type
                if !global_book.items.contains_key(&item_type) {
                    global_book
                        .items
                        .insert(item_type, OrderBook::new(item_type));
                }

                if let Some(mut order_book) = global_book.items.get_mut(&item_type) {
                    for order in modified_orders {
                        order_book.orders.insert(order.id, order);
                        modified_ordercount += 1;
                    }
                }
            }

            println!(
                "Applied delta for region {} - {new_ordercount} new orders, {modified_ordercount} modified orders, {removed_ordercount} removed orders",
                region.name
            );

            // Update global market timestamps if this market is newer
            if new_last_modified > global_book.last_modified {
                global_book.last_modified = new_last_modified;
            }
            if new_expires > global_book.expires {
                global_book.expires = new_expires;
            }

            // Release the global book lock
            drop(global_book);

            // Store the new regional market data
            regions.insert(region, new_market);
        });
    }
}
