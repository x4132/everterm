use axum::{Router};

use backend::market_data;

#[tokio::main]
async fn main() {
    let api_routes = Router::new()
        .merge(market_data());

    let app = Router::new()
        .nest("/api", api_routes);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await.unwrap();
    axum::serve(listener,app).await.unwrap();
}