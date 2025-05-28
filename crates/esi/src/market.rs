use chrono::{DateTime, ParseError, Utc};
use dashmap::DashMap;
use reqwest::header::{EXPIRES, HeaderValue, LAST_MODIFIED};
use serde::{
    Deserialize,
    de::{self, Visitor},
};
use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::{self},
    sync::Arc,
};
use tokio::sync::Mutex;

use crate::{
    universe::{InvalidIDError, Region, StationID, SystemID}, ESIClient
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

#[derive(Debug, Clone)]
pub struct MarketDiff {
    pub new: HashMap<u32, Vec<Order>>,
    pub modified: HashMap<u32, Vec<Order>>,
    pub removed: HashMap<u32, Vec<u64>>,
}

impl MarketDiff {
    pub fn new() -> Self {
        MarketDiff {
            new: HashMap::new(),
            modified: HashMap::new(),
            removed: HashMap::new(),
        }
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

#[derive(Clone, Deserialize, Debug, PartialEq)]
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
        let location_id =
            StationID::try_from(value.location_id).map_err(OrderConversionError::InvalidIDError)?;

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
    pub orders: HashMap<u64, Order>,
}
impl OrderBook {
    pub fn new(item: u32) -> Self {
        OrderBook {
            item,
            orders: HashMap::new(),
        }
    }

    pub fn merge(&mut self, other: Self) -> Result<(), InvalidIDError> {
        if self.item != other.item {
            return Err(InvalidIDError {
                value: other.item.into(),
                acceptable: self.item.into()..self.item.into(),
            });
        }

        self.orders.extend(other.orders);

        Ok(())
    }
}

#[derive(Debug)]
pub struct Market {
    pub items: DashMap<u32, OrderBook>,
    pub last_modified: DateTime<Utc>,
    pub expires: DateTime<Utc>,
}

impl Market {
    pub fn new() -> Self {
        Market {
            items: DashMap::new(),
            last_modified: DateTime::UNIX_EPOCH,
            expires: DateTime::UNIX_EPOCH,
        }
    }

    /// loads the market orders of a region.
    pub async fn fetch_regions(
        regions: Vec<Region>,
        client: Arc<ESIClient>,
    ) -> anyhow::Result<Self> {
        println!(
            "Markets: Starting region orderbook fetching at {}",
            chrono::Utc::now()
        );
        let mut market = Market::new();

        let markets = Arc::new(Mutex::new(Vec::new()));
        let mut handles = Vec::new();
        for region in regions {
            let markets = markets.clone();
            let client = client.clone();
            let handle = tokio::spawn(async move {
                let market = Self::fetch_region(&region, client).await.unwrap();
                markets.lock().await.push(market);
            });

            handles.push(handle);
        }

        futures::future::try_join_all(handles).await?;

        let markets = Arc::try_unwrap(markets)
            .expect("Arc still has multiple strong counts")
            .into_inner();
        println!(
            "Markets: Finished fetching orderbooks at {}",
            chrono::Utc::now()
        );

        for region in markets {
            if market.last_modified == DateTime::UNIX_EPOCH {
                market.last_modified = region.last_modified;
                market.expires = region.expires;
            }

            for item in region.items {
                if !market.items.contains_key(&item.0) {
                    market.items.insert(item.0, OrderBook::new(item.0));
                }

                market
                    .items
                    .get_mut(&item.0)
                    .unwrap()
                    .merge(item.1)
                    .unwrap();
            }
        }

        return Ok(market);
    }

    pub async fn fetch_region(region: &Region, client: Arc<ESIClient>) -> anyhow::Result<Self> {
        println!("Markets: Fetching Orderbook for {}", region.name);
        let first_page = client
            .esi_get(&format!("/markets/{}/orders/", region.id.get()))
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
            let region_id = region.id.get();
            let handle = tokio::spawn(async move {
                let page = client
                    .esi_get(&format!("/markets/{}/orders/?page={}", region_id, page))
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
            match Order::try_from(order_response) {
                Ok(order) => {
                    let order_id = order.id;
                    market
                        .items
                        .get_mut(&type_id)
                        .unwrap()
                        .orders
                        .insert(order_id, order);
                }
                Err(err) => {
                    eprintln!("{:?}", err);
                }
            }
        }

        println!(
            "Markets: Finished fetching orderbook for region {}",
            region.name
        );
        Ok(market)
    }

    /// This function compares two markets and returns the diff between the two.
    pub fn delta(&self, new_market: &Self) -> MarketDiff {
        let mut diff = MarketDiff::new();

        // go through existing items and find updates
        for old_item in self.items.iter() {
            match new_market.items.get(old_item.key()) {
                Some(new_orderbook) => {
                    let new_ordermap = &new_orderbook.orders;

                    diff.modified.insert(old_item.item, Vec::new());
                    diff.removed.insert(old_item.item, Vec::new());

                    // Check for modified/unchanged orders and removed orders
                    for old_order in old_item.orders.iter() {
                        match new_ordermap.get(old_order.0) {
                            Some(other_order) => {
                                if other_order != old_order.1 {
                                    // Order was modified
                                    diff.modified
                                        .get_mut(old_item.key())
                                        .unwrap()
                                        .push(other_order.clone());
                                }
                                // If they're equal, the order is unchanged (no action needed)
                            }
                            None => {
                                // The order was cancelled/filled/removed in some way
                                diff.removed.get_mut(old_item.key()).unwrap().push(old_order.1.id);
                            }
                        }
                    }

                    // Check for new orders in the other market
                    for other_order in new_ordermap.iter() {
                        if !old_item.orders.contains_key(other_order.0) {
                            // This is a new order
                            if !diff.new.contains_key(old_item.key()) {
                                diff.new.insert(old_item.item, Vec::new());
                            }
                            diff.new
                                .get_mut(old_item.key())
                                .unwrap()
                                .push(other_order.1.clone());
                        }
                    }
                }

                None => {
                    // This item category is GONE in the new market
                    diff.removed.insert(old_item.item, Vec::new());
                    let item_vec = diff.removed.get_mut(old_item.key()).unwrap();
                    for removed_order in old_item.orders.iter() {
                        item_vec.push(removed_order.1.id);
                    }
                }
            }
        }

        // Find completely new item categories
        for item in new_market.items.iter() {
            if !self.items.contains_key(item.key()) {
                diff.new.insert(
                    item.item,
                    item.orders.iter().map(|order| order.1.clone()).collect(),
                );
            }
        }

        return diff;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universe::{StationID, SystemID};
    use chrono::{Duration, TimeZone, Utc};

    fn make_order(id: u64, price: f64) -> Order {
        let issued = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let expiry = issued + Duration::days(1);
        let location_id = StationID::try_from(60_000_001).unwrap();
        let system_id = SystemID::try_from(30_000_001).unwrap();
        Order {
            id,
            is_buy_order: false,
            price,
            issued,
            expiry,
            location_id,
            system_id,
            min_volume: 1,
            range: MarketOrderRange::Region,
            volume_remain: 1,
            volume_total: 1,
        }
    }

    #[test]
    fn test_delta_empty() {
        let m1 = Market::new();
        let m2 = Market::new();
        let diff = m1.delta(&m2);
        assert!(diff.new.is_empty());
        assert!(diff.modified.is_empty());
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn test_delta_removed() {
        let m1 = Market::new();
        let mut book = OrderBook::new(100);
        let o = make_order(1, 10.0);
        book.orders.insert(o.id, o.clone());
        m1.items.insert(100, book);
        let m2 = Market::new();
        let diff = m1.delta(&m2);
        assert_eq!(diff.removed.get(&100).unwrap(), &vec![1]);
        assert!(diff.new.get(&100).is_none());
        assert!(diff.modified.get(&100).is_none());
    }

    #[test]
    fn test_delta_new() {
        let m1 = Market::new();
        let m2 = Market::new();
        let mut book = OrderBook::new(200);
        let o = make_order(2, 20.0);
        book.orders.insert(o.id, o.clone());
        m2.items.insert(200, book);
        let diff = m1.delta(&m2);
        assert_eq!(diff.new.get(&200).unwrap(), &vec![o]);
        assert!(diff.modified.get(&200).is_none());
        assert!(diff.removed.get(&200).is_none());
    }

    #[test]
    fn test_delta_modified() {
        let m1 = Market::new();
        let m2 = Market::new();
        let mut b1 = OrderBook::new(300);
        let o1 = make_order(3, 30.0);
        b1.orders.insert(o1.id, o1.clone());
        m1.items.insert(300, b1);
        let mut b2 = OrderBook::new(300);
        let o2 = make_order(3, 35.0);
        b2.orders.insert(o2.id, o2.clone());
        m2.items.insert(300, b2);
        let diff = m1.delta(&m2);
        assert!(diff.new.get(&300).is_none());
        assert!(diff.removed.get(&300).unwrap().is_empty());
        assert_eq!(diff.modified.get(&300).unwrap(), &vec![o2]);
    }

    #[test]
    fn test_delta_mixed_operations() {
        let m1 = Market::new();
        let m2 = Market::new();

        // Add an item with multiple orders to m1
        let mut b1 = OrderBook::new(100);
        let o1 = make_order(1, 10.0);
        let o2 = make_order(2, 20.0);
        b1.orders.insert(o1.id, o1.clone());
        b1.orders.insert(o2.id, o2.clone());
        m1.items.insert(100, b1);

        // Add same item to m2 with one modified order, one unchanged, and one new
        let mut b2 = OrderBook::new(100);
        let o1_unchanged = o1.clone(); // same order
        let o2_modified = make_order(2, 25.0); // modified price
        let o3_new = make_order(3, 30.0); // new order
        b2.orders.insert(o1_unchanged.id, o1_unchanged);
        b2.orders.insert(o2_modified.id, o2_modified.clone());
        b2.orders.insert(o3_new.id, o3_new.clone());
        m2.items.insert(100, b2);

        let diff = m1.delta(&m2);

        // Should have one modified order and one new order
        assert_eq!(diff.modified.get(&100).unwrap(), &vec![o2_modified]);
        assert_eq!(diff.new.get(&100).unwrap(), &vec![o3_new]);
        assert!(diff.removed.get(&100).unwrap().is_empty());
    }

    #[test]
    fn test_delta_unchanged_orders() {
        let m1 = Market::new();
        let m2 = Market::new();

        let mut b1 = OrderBook::new(100);
        let o1 = make_order(1, 10.0);
        b1.orders.insert(o1.id, o1.clone());
        m1.items.insert(100, b1);

        let mut b2 = OrderBook::new(100);
        let o1_same = o1.clone(); // exactly the same order
        b2.orders.insert(o1_same.id, o1_same);
        m2.items.insert(100, b2);

        let diff = m1.delta(&m2);

        // No changes should be detected
        assert!(diff.new.get(&100).is_none());
        assert!(diff.modified.get(&100).unwrap().is_empty());
        assert!(diff.removed.get(&100).unwrap().is_empty());
    }
}
