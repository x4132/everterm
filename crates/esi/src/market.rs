use chrono::{DateTime, ParseError, Utc};
use dashmap::DashMap;
use reqwest::header::{EXPIRES, HeaderValue, LAST_MODIFIED};
use serde::{
    Deserialize,
    de::{self, Visitor},
};
use std::{
    cmp::Ordering,
    collections::BTreeSet,
    fmt::{self},
    sync::Arc,
};
use tokio::sync::Mutex;

use crate::{
    ESIClient,
    universe::{InvalidIDError, Item, RegionID, Regions, StationID, SystemID},
};

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

#[derive(Deserialize, Debug, Clone)]
struct MarketAPIResponseOrder {
    duration: u32,
    is_buy_order: bool,
    issued: String,
    location_id: u64,
    min_volume: u32,
    order_id: u64,
    price: f64,
    range: MarketOrderRange,
    system_id: SystemID,
    type_id: u32,
    volume_remain: u32,
    volume_total: u32,
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Order {
    pub id: u64,
    pub is_buy_order: bool,
    pub price: f64,
    pub issued: DateTime<Utc>,
    pub expiry: DateTime<Utc>,
    pub location_id: StationID,
    pub system_id: SystemID,
    pub min_volume: u32,
    pub range: MarketOrderRange,
    pub volume_remain: u32,
    pub volume_total: u32,
}

impl Eq for Order {}

impl PartialOrd for Order {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Order {
    fn cmp(&self, other: &Self) -> Ordering {
        // Descending by price: highest price first
        self.price.total_cmp(&other.price).reverse()
    }
}

#[derive(Debug)]
pub enum OrderConversionError {
    ParseError(ParseError),
    InvalidIDError(InvalidIDError),
}

impl TryFrom<MarketAPIResponseOrder> for Order {
    type Error = OrderConversionError;

    fn try_from(value: MarketAPIResponseOrder) -> Result<Self, Self::Error> {
        let issue_date = DateTime::parse_from_rfc3339(&value.issued)
            .map_err(OrderConversionError::ParseError)?
            .to_utc();
        let location_id = StationID::try_from(value.location_id).map_err(OrderConversionError::InvalidIDError)?;

        Ok(Order {
            id: value.order_id,
            is_buy_order: value.is_buy_order,
            price: value.price,
            issued: issue_date,
            expiry: issue_date + chrono::TimeDelta::days(value.duration.into()),
            location_id,
            system_id: value.system_id,
            min_volume: value.min_volume,
            range: value.range,
            volume_total: value.volume_total,
            volume_remain: value.volume_remain,
        })
    }
}

/// Carries the current orders at a single snapshot.
#[derive(Clone, Debug)]
pub struct OrderBook {
    pub item: u32,
    pub orders: BTreeSet<Order>,
}
impl OrderBook {
    pub fn new(item: u32) -> Self {
        OrderBook {
            item,
            orders: BTreeSet::new(),
        }
    }
}

#[derive(Debug)]
pub struct Market {
    pub items: DashMap<u32, OrderBook>,
    pub last_modified: DateTime<Utc>,
    pub expires: DateTime<Utc>,
}

impl Market {
    /// loads the market orders of a region.
    // pub async fn fetch_regions(regions: Vec<RegionID>, client: ESIClient) -> Result<Self, Box<dyn std::error::Error>> {
    //     let market = Market {
    //         items: DashMap::new(),
    //         time: DateTime::UNIX_EPOCH
    //     };

    //     for region in regions {

    //     }
    // }

    pub async fn fetch_region(
        region: RegionID,
        client: Arc<ESIClient>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let first_page = client
            .esi_get(&format!("/markets/{}/orders/", region.get()))
            .await?;
        let first_page_headers = first_page.headers();
        let num_pages: usize = first_page_headers
            .get("x-pages")
            .unwrap_or(&HeaderValue::from_static("1"))
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        let last_modified: DateTime<Utc> = DateTime::parse_from_rfc2822(
            first_page_headers
                .get(LAST_MODIFIED)
                .expect("No LAST_MODIFIED header in market data response?")
                .to_str()?,
        )?
        .to_utc();
        let expires: DateTime<Utc> = DateTime::parse_from_rfc2822(
            first_page_headers
                .get(EXPIRES)
                .expect("No EXPIRES header in market data response?")
                .to_str()?,
        )?
        .to_utc();

        let pages: Vec<MarketAPIResponseOrder> =
            first_page.json::<Vec<MarketAPIResponseOrder>>().await?;
        let orders: Arc<Mutex<Vec<MarketAPIResponseOrder>>> = Arc::new(Mutex::new(pages));
        let client = Arc::new(client);
        let mut handles = Vec::new();
        for page in 2..=num_pages {
            let pages = orders.clone();
            let client = client.clone();
            let handle = tokio::spawn(async move {
                let page = client
                    .esi_get(&format!("/markets/{}/orders/?page={}", region.get(), page))
                    .await
                    .expect("Failed to load page")
                    .json::<Vec<MarketAPIResponseOrder>>()
                    .await
                    .expect("Failed to deserialize JSON");
                let mut pages = pages.lock().await;
                pages.extend(page.into_iter());
            });

            handles.push(handle);
        }

        let market = Market {
            items: DashMap::new(),
            last_modified,
            expires,
        };

        futures::future::try_join_all(handles).await?;

        let orders = Arc::try_unwrap(orders)
            .expect("Arc still has multiple strong counts")
            .into_inner();
        for order_response in orders {
            if !(&market).items.contains_key(&order_response.type_id) {
                market.items.insert(
                    order_response.type_id,
                    OrderBook::new(order_response.type_id),
                );
            }

            let type_id = order_response.type_id;
            if let Ok(order) = Order::try_from(order_response) {
                market.items.get_mut(&type_id).unwrap().orders.insert(order);
            }
        }

        Ok(market)
    }
}
