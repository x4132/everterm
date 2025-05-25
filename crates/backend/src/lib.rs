use axum::{extract::Path, routing::get, Router};

pub fn market_data() -> Router {
    Router::new()
        .route("/orders/{id}", get(get_order))
}

async fn get_order(Path(id): Path<String>) -> String {
    reqwest::get(format!("https://evetycoon.com/api/v1/market/orders/{id}")).await.unwrap().text().await.unwrap()
}