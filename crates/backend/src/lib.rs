use axum::{extract::Path, routing::get, Router};

static DATAFETCH_URL: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| std::env::var("DATAFETCH_URL").unwrap_or(String::from("http://0.0.0.0:6380")));

pub fn market_data() -> Router {
    Router::new()
        .route("/orders/{id}", get(get_order))
}

async fn get_order(Path(id): Path<String>) -> String {
    reqwest::get(format!("{}/market/{id}", DATAFETCH_URL.to_owned())).await.unwrap().text().await.unwrap()
}