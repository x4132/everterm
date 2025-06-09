use std::{collections::{HashMap, HashSet}, env, sync::Arc};

use axum::{
    Router,
    extract::{Path, Query, State},
    response::Response,
    routing::get,
};
use esi::{
    universe::{self, StationID, Stations, Structure}, ESIClient
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
    public_structs: Arc<RwLock<HashSet<StationID>>>,
}

pub async fn market_data() -> Router {
    let esi_client = Arc::new(RwLock::new(ESIClient::new(
        "backend",
        std::env::consts::OS,
        8,
    )));
    let stations_client = Arc::new(ESIClient::new("backend", std::env::consts::OS, 8));

    let mut allowed: HashSet<StationID> = esi_client.read().await.esi_get("/universe/structures/?datasource=tranquility&filter=market").await.unwrap().json().await.unwrap();
    allowed.insert(StationID::try_from(1042508032148).unwrap());
    allowed.insert(StationID::try_from(1042499803831).unwrap());
    allowed.insert(StationID::try_from(1042225270361).unwrap());
    allowed.insert(StationID::try_from(1031058135975).unwrap());
    allowed.insert(StationID::try_from(1043817963096).unwrap());
    allowed.insert(StationID::try_from(1033368129183).unwrap());
    allowed.insert(StationID::try_from(1043439680449).unwrap());
    allowed.insert(StationID::try_from(1045198535020).unwrap());
    allowed.insert(StationID::try_from(1043601200296).unwrap());
    allowed.insert(StationID::try_from(1043407541875).unwrap());
    allowed.insert(StationID::try_from(1049346544892).unwrap());
    allowed.insert(StationID::try_from(1048937166442).unwrap());
    allowed.insert(StationID::try_from(1048745091799).unwrap());
    allowed.insert(StationID::try_from(1036835214921).unwrap());
    allowed.insert(StationID::try_from(1042847222396).unwrap());
    allowed.insert(StationID::try_from(1048983558976).unwrap());
    allowed.insert(StationID::try_from(1042176139111).unwrap());
    allowed.insert(StationID::try_from(1047339254410).unwrap());
    allowed.insert(StationID::try_from(1036753634403).unwrap());
    allowed.insert(StationID::try_from(1045193582746).unwrap());
    allowed.insert(StationID::try_from(1032717532381).unwrap());
    allowed.insert(StationID::try_from(1043159223409).unwrap());
    allowed.insert(StationID::try_from(1037279288949).unwrap());
    allowed.insert(StationID::try_from(1049037316814).unwrap());
    allowed.insert(StationID::try_from(1031084757448).unwrap());
    allowed.insert(StationID::try_from(1047753226436).unwrap());
    allowed.insert(StationID::try_from(1032715081490).unwrap());
    allowed.insert(StationID::try_from(1044151786607).unwrap());
    allowed.insert(StationID::try_from(1047163531118).unwrap());
    allowed.insert(StationID::try_from(1043441907485).unwrap());
    allowed.insert(StationID::try_from(1033196707294).unwrap());
    allowed.insert(StationID::try_from(1044189620907).unwrap());
    allowed.insert(StationID::try_from(1047143806189).unwrap());
    allowed.insert(StationID::try_from(1037962518481).unwrap());
    allowed.insert(StationID::try_from(1046351183405).unwrap());
    allowed.insert(StationID::try_from(1039827685477).unwrap());
    allowed.insert(StationID::try_from(1044164309897).unwrap());
    allowed.insert(StationID::try_from(1033050967689).unwrap());
    allowed.insert(StationID::try_from(1025824394754).unwrap());
    allowed.insert(StationID::try_from(1039479389173).unwrap());
    allowed.insert(StationID::try_from(1041517397230).unwrap());

    let state = AppState {
        esi_client: esi_client.clone(),
        stations: Arc::new(Stations::new(stations_client)),
        public_structs: Arc::new(RwLock::new(allowed)),
    };


    Router::new()
        .route(
            "/ping",
            get(|| async { format!("OK {}", chrono::Utc::now().to_rfc2822()) }),
        )
        .route("/orders/{id}", get(get_orders))
        .route("/orders/updateTime", get(get_update_time))
        .route("/universe/struct_names/", get(get_structures))
        .with_state(state)
}

async fn get_orders(Path(id): Path<String>) -> Result<Response, StatusCode> {
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

async fn get_update_time() -> Result<Response, StatusCode> {
    let response = reqwest::get(format!("{}/refresh_intervals", DATAFETCH_URL.to_owned()))
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

    if let Ok(station_id) = StationID::try_from(id) {
        if station_id.get() < 64_000_000 {
            let station = state.stations.get_station(station_id).await.unwrap();
            let builder = Response::builder().status(StatusCode::OK);

            Ok(builder
                .body(serde_json::to_string(&station).unwrap().into())
                .unwrap())
        } else {
            let structure;

            if state.public_structs.read().await.contains(&station_id) {
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

                let esi_client = state.esi_client.read().await;

                // we just depend on http cacache for this one
                let req = esi_client
                    .esi_get(&format!("/universe/structures/{}/", station_id.get()))
                    .await
                    .unwrap()
                    .json::<universe::StructureAPIResponse>()
                    .await
                    .unwrap();

                structure = universe::Structure {
                    id: station_id,
                    name: req.name,
                    system_id: req.system_id,
                    type_id: req.type_id,
                };
            } else {
                structure = universe::Structure {
                    id: station_id,
                    name: String::from("Unknown Private Structure"),
                    system_id: universe::SystemID::try_from(30000380).unwrap(),
                    type_id: 0,
                };
            }

            let builder = Response::builder().status(StatusCode::OK);

            Ok(builder
                .body(serde_json::to_string(&structure).unwrap().into())
                .unwrap())
        }
    } else {
        return Err(StatusCode::BAD_REQUEST);
    }
}
