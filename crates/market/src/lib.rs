use chrono::{DateTime, Utc};
use universe::System;

pub enum MarketOrderRange {
    System(u32),
    Station,
    Region
}

pub struct MarketOrder {
    id: u64,
    // duration: u32, - this is not really relevant for any user, there is no reason to know when it expires over when it was issued
    is_buy_order: bool,
    issued: DateTime<Utc>,
    location: System,
    // type:
    min_volume: u32,
    range: MarketOrderRange,
    volume_remain: u32,
    volume_total: u32,
}

// mod tests {
//     use super::*;

//     #[test]
//     fn fetch_market_orders() {

//    }
// }