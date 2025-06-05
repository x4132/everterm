use std::{env, sync::Arc};

use axum::{Json, Router, extract::Path, response::IntoResponse, routing::get};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use esi::market::{Market, Order};
use tokio::sync::Mutex;

pub async fn data_server(
    refresh_intervals: Arc<DashMap<u32, Option<DateTime<Utc>>>>,
    market: Arc<Mutex<Market>>,
) -> Result<(), std::io::Error> {
    let server = Router::new()
        .route(
            "/ping",
            get(|| async { format!("OK {}", Utc::now().to_rfc2822()) }),
        )
        .route("/refresh_intervals", {
            let refresh_intervals = refresh_intervals.clone();
            get(move || async move { serde_json::to_string(&*refresh_intervals).unwrap() })
        })
        .route("/market/{id}", {
            let market = market.clone();
            get(move |Path(id): Path<String>| async move {
                let id = id.parse::<u32>();
                if id.is_err() {
                    return (axum::http::StatusCode::BAD_REQUEST, "Invalid ID format")
                        .into_response();
                }
                let id = id.unwrap();

                match market.lock().await.items.get(&id) {
                    Some(orderbook) => {
                        let orders: Vec<Order> = orderbook.value().orders.clone().into_values().collect();
                        Json(orders).into_response()
                    },
                    None => (axum::http::StatusCode::NOT_FOUND, "Item Type Not Found").into_response()
                }
            })
        });

    let tcp_listener = tokio::net::TcpListener::bind(format!(
        "0.0.0.0:{}",
        env::var("PORT").unwrap_or(String::from("6380"))
    ))
    .await
    .unwrap();

    axum::serve(tcp_listener, server).await
}
