use std::{collections::HashMap, env, sync::Arc};

use axum::{
    Router,
    extract::{Path, Query, State},
    response::Response,
    routing::get,
};
use esi::{
    ESIClient,
    universe::{Station, StationID, Stations},
};
use reqwest::{StatusCode, header};
use tokio::sync::RwLock;

static DATAFETCH_URL: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    std::env::var("DATAFETCH_URL").unwrap_or(String::from("http://0.0.0.0:6380"))
});

#[derive(Clone)]
struct AppState {
    esi_client: Arc<RwLock<ESIClient>>,
    stations: Arc<Stations>,
}

pub fn market_data() -> Router {
    let esi_client = Arc::new(RwLock::new(ESIClient::new(
        "backend",
        std::env::consts::OS,
        8,
    )));
    let stations_client = Arc::new(ESIClient::new("backend", std::env::consts::OS, 8));

    let state = AppState {
        esi_client: esi_client.clone(),
        stations: Arc::new(Stations::new(stations_client)),
    };

    Router::new()
        .route(
            "/ping",
            get(|| async { format!("OK {}", chrono::Utc::now().to_rfc2822()) }),
        )
        .route("/orders/{id}", get(get_order))
        .route("/universe/struct_names/", get(get_structures))
        .with_state(state)
}

async fn get_order(Path(id): Path<String>) -> Result<Response, StatusCode> {
    let response = reqwest::get(format!("{}/market/{id}", DATAFETCH_URL.to_owned()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let builder = Response::builder()
        .status(response.status())
        .header(header::CONTENT_TYPE, "application/json");

    let body = response
        .bytes()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(builder.body(body.into()).unwrap())
}

async fn get_structures(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, StatusCode> {
    if !params.contains_key("id") {
        return Err(StatusCode::BAD_REQUEST);
    }


    let id: u64 = params
        .get("id")
        .ok_or(StatusCode::BAD_REQUEST)?
        .parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let station: Station;
    if let Ok(station_id) = StationID::try_from(id) {
        if station_id.get() < 64_000_000 {
            station = state
                .stations
                .get_station(station_id)
                .await.unwrap();
        } else {
            // we need a database of all the data for public structures lol
            dotenvy::dotenv().unwrap();

            // Check if auth token is valid with read lock first
            let auth_valid = {
                let esi_client = state.esi_client.read().await;
                esi_client.auth_tok_valid().await
            };

            // Only acquire write lock if we need to update the token
            if !auth_valid {
                let mut esi_client = state.esi_client.write().await;
                esi_client
                    .load_auth_tok(
                        env::var("PUB_STRUCT_ESI_REFRESH").unwrap(),
                        env::var("CLIENT_ID").unwrap(),
                        env::var("CLIENT_SECRET").unwrap(),
                    )
                    .await
                    .unwrap();
            }

            // Need to handle the structure case - for now return an error
            return Err(StatusCode::NOT_IMPLEMENTED);
        }
    } else {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }

    println!("{station:?}");
    let builder = Response::builder().status(StatusCode::OK);

    Ok(builder
        .body(serde_json::to_string(&station).unwrap().into())
        .unwrap())
}
