use chrono::{DateTime, Utc};

pub struct MarketOrder {
    duration: u32,
    is_buy_order: bool,
    issued: DateTime<Utc>,
    // location: ,
}

// mod tests {
//     use super::*;

//     #[test]
//     fn fetch_market_orders() {

//    }
// }