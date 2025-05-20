use chrono::{DateTime, Utc};
use reqwest::header::{EXPIRES, LAST_MODIFIED};
use serde::{
    Deserialize,
    de::{self, Visitor},
};
use std::{
    error::Error,
    fmt::{self},
    sync::Arc,
};
use tokio::sync::Mutex;
use universe::{Item, Region, Station, esi};

#[derive(Clone, PartialEq, Debug)]
pub enum MarketOrderRange {
    System(u32),
    Station,
    Region,
}

impl<'de> Deserialize<'de> for MarketOrderRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MarketOrderRangeVisitor;

        impl<'de> Visitor<'de> for MarketOrderRangeVisitor {
            type Value = MarketOrderRange;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(r#""station", "region", or a number"#)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    "station" => Ok(MarketOrderRange::Station),
                    "region" => Ok(MarketOrderRange::Region),
                    "solarsystem" => Ok(MarketOrderRange::System(1)),
                    _ => {
                        let num_range: Result<u32, _> = value.parse();
                        match num_range {
                            Ok(val) => Ok(MarketOrderRange::System(val)),
                            Err(_) => Err(E::custom(format!("unexpected string: {}", value))),
                        }
                    }
                }
            }
        }

        deserializer.deserialize_any(MarketOrderRangeVisitor)
    }
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct MarketOrder {
    pub id: u64,
    // duration: u32, - this is not really relevant for any user, there is no reason to know when it expires over when it was issued
    pub is_buy_order: bool,
    pub issued: DateTime<Utc>,
    pub location: Station,
    pub min_volume: u32,
    pub range: MarketOrderRange,
    pub volume_remain: u32,
    pub volume_total: u32,
}

impl MarketOrder {
    async fn from_api_response(value: MarketAPIResponseOrder) -> Result<Self, Box<dyn Error>> {
        let issued = DateTime::parse_from_rfc3339(&value.issued)?.to_utc();
        // println!("Attempting structure {}", value.location_id);

        Ok(MarketOrder {
            id: value.order_id,
            is_buy_order: value.is_buy_order,
            issued: issued,
            location: Station::get_station(value.location_id).await?,
            min_volume: value.min_volume,
            range: value.range,
            volume_remain: value.volume_remain,
            volume_total: value.volume_total,
        })
    }
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct ItemOrderBook {
    pub item: Item,
    pub orders: Vec<MarketOrder>,
    pub time: Option<DateTime<Utc>>,
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct MarketOrderBook {
    pub items: Vec<ItemOrderBook>,
    pub time: DateTime<Utc>,
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct MarketRegion {
    pub region: Region,
    pub orders: Vec<MarketOrder>,
    pub cache_reset_time: DateTime<Utc>,
    pub fetch_time: DateTime<Utc>,
}

impl MarketRegion {
    pub fn new_with_orders(
        region: Region,
        orders: Vec<MarketOrder>,
        cache_reset_time: DateTime<Utc>,
        fetch_time: DateTime<Utc>,
    ) -> Self {
        MarketRegion {
            region,
            orders,
            cache_reset_time,
            fetch_time,
        }
    }
    pub fn new(region: Region, cache_reset_time: DateTime<Utc>, fetch_time: DateTime<Utc>) -> Self {
        MarketRegion::new_with_orders(region, Vec::new(), cache_reset_time, fetch_time)
    }
}

#[derive(Deserialize, Debug)]
struct MarketAPIResponseOrder {
    duration: u32,
    is_buy_order: bool,
    issued: String,
    location_id: u64,
    min_volume: u32,
    order_id: u64,
    price: f64,
    range: MarketOrderRange,
    system_id: u32,
    type_id: u32,
    volume_remain: u32,
    volume_total: u32,
}

impl MarketRegion {
    pub async fn fetch_orders(region: Region) -> Result<MarketRegion, Box<dyn Error>> {
        println!("Loading orders for {}", region.name);
        let page_1 = reqwest::get(esi!("/markets/{}/orders", region.region_id)).await?;
        let headers = page_1.headers();
        let pages: u32 = headers
            .get("x-pages")
            .expect("Failed to retrieve X-Pages header on region market orders")
            .to_str()
            .unwrap()
            .parse()
            .expect("X-Pages header invalid file format");
        let last_modified = DateTime::parse_from_rfc2822(
            headers
                .get(LAST_MODIFIED)
                .expect("Failed to retreive last-modified header")
                .to_str()
                .unwrap(),
        )
        .expect("Failed to parse last-modified datetime string")
        .to_utc();
        let expires = DateTime::parse_from_rfc2822(
            headers
                .get(EXPIRES)
                .expect("Failed to retreive expiry date")
                .to_str()
                .unwrap(),
        )
        .expect("Failed to parse expiry date")
        .to_utc();

        println!(
            "Loading {pages} pages for the region \"{}\", last updated {}, expires {}",
            region.name,
            last_modified.to_string(),
            expires.to_string()
        );

        let orders = page_1.json::<Vec<MarketAPIResponseOrder>>().await?;
        let orders = Arc::new(Mutex::new(orders));
        let mut handles = Vec::new();

        for page in 2..(pages+1) {
            let orders = orders.clone();
            let handle = tokio::spawn(async move {
                let page = reqwest::get(esi!("/markets/{}/orders?page={}", region.region_id, page))
                    .await
                    .expect(&format!("Failed to query page {page}"))
                    .json::<Vec<MarketAPIResponseOrder>>()
                    .await
                    .unwrap();
                let mut orders = orders.lock().await;
                orders.extend(page);
            });

            handles.push(handle);
        }

        futures::future::try_join_all(handles)
            .await
            .expect("Error retreiving orders for market region");

        let orders = Arc::try_unwrap(orders)
            .expect("Arc still has multiple strong counts")
            .into_inner();

        println!("Converting {} orders", orders.len());

        let region = Arc::new(std::sync::Mutex::new(MarketRegion::new(
            region,
            expires,
            last_modified,
        )));
        let mut handles = Vec::new();
        for order in orders {
            let region = region.clone();
            let handle = tokio::spawn(async move {
                match MarketOrder::from_api_response(order).await {
                    Ok(order) => {
                        region.lock().expect("Unable to acquire lock on regional orders log").orders.push(order);
                    }
                    Err(err) => {
                        if !err.is::<universe::StationError>() {
                            println!("Failed to convert order");
                            eprintln!("{err}");
                        }
                    }
                };
            });
            handles.push(handle);
        }
        futures::future::try_join_all(handles)
            .await
            .expect("Error formatting Market Orders");

        Ok(Arc::try_unwrap(region).expect("Arc still has multiple strong counts").into_inner().expect("Regional Orders Mutex is Poisoned"))
    }
}

// mod tests {
//     use super::*;

//     #[test]
//     fn fetch_market_orders() {

//    }
// }
