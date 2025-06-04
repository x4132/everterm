use axum::{Router, extract::Path, routing::get};
use esi::ESIClient;

static DATAFETCH_URL: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    std::env::var("DATAFETCH_URL").unwrap_or(String::from("http://0.0.0.0:6380"))
});

static ESI_CLIENT: std::sync::LazyLock<ESIClient> =
    std::sync::LazyLock::new(|| ESIClient::new("backend", std::env::consts::OS, 8));

pub fn market_data() -> Router {
    Router::new()
        .route(
            "/ping",
            get(|| async { format!("OK {}", chrono::Utc::now().to_rfc2822()) }),
        )
        .route("/orders/{id}", get(get_order))
        .route("/universe/struct_names/", get(get_structures))
}

async fn get_order(Path(id): Path<String>) -> String {
    reqwest::get(format!("{}/market/{id}", DATAFETCH_URL.to_owned()))
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
}

async fn get_structures() -> String {
    if ESI_CLIENT.
    String::from("{}")
}
