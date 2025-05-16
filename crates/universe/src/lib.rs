use std::error::Error;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, OnceCell};

use serde::Deserialize;

mod macros;

/// This struct represents a geospatial point in the EvE universe.
#[derive(Clone, Debug, Copy, Deserialize, PartialEq)]
pub struct Point {
    x: f64,
    y: f64,
    z: f64,
}
impl Point {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Point { x, y, z }
    }
}

/**
========================================
REGION API
========================================
*/

static REGIONS: OnceCell<Arc<Mutex<Regions>>> = OnceCell::const_new();

async fn regions_cache_init() -> Arc<Mutex<Regions>> {
    Arc::new(Mutex::new(Regions::new()))
}

/// This struct represents a region in the EvE universe.
#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Region {
    pub id: u32,
    pub name: String,
}

impl Region {
    pub async fn get_region(id: u32) -> RegionResult {
        let cache = REGIONS.get_or_init(regions_cache_init).await;

        {
            let cache = cache.lock().await;
            if let Some(data) = cache.0.get(&id) {
                return Ok(data.clone());
            }
        }

        Region::fetch_region(id).await
    }

    async fn fetch_region(id: u32) -> RegionResult {
        let region = reqwest::get(esi!("/universe/regions/{}", id))
            .await?
            .json::<RegionAPIRequest>()
            .await?;

        let region = Region::from(region);

        let mut cache = REGIONS.get_or_init(regions_cache_init).await.lock().await;
        cache.0.insert(region.id, region.clone());

        Ok(region)
    }
}

impl From<RegionAPIRequest> for Region {
    fn from(api_request: RegionAPIRequest) -> Self {
        Region {
            id: api_request.region_id,
            name: api_request.name,
        }
    }
}

type RegionResult = Result<Region, Box<dyn Error>>;

#[derive(Deserialize)]
struct RegionAPIRequest {
    region_id: u32,
    name: String,
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Regions(pub HashMap<u32, Region>);

impl Regions {
    pub fn new() -> Self {
        Regions(HashMap::new())
    }

    pub async fn fetch_all() -> Result<Regions, Box<dyn std::error::Error>> {
        let ids = reqwest::get(esi!("/universe/regions/"))
            .await?
            .json::<Vec<u32>>()
            .await?;
        let regions_map = Arc::new(Mutex::new(HashMap::new()));
        let mut handles = Vec::new();

        for id in ids {
            let regions_map = regions_map.clone();
            let handle = tokio::spawn(async move {
                let info = Region::fetch_region(id).await.unwrap();

                let mut regions_map = regions_map.lock().await;
                regions_map.insert(info.id, info);
            });

            handles.push(handle);
        }

        futures::future::try_join_all(handles)
            .await
            .expect("Error in one of the spawned tasks");

        let final_map = Arc::try_unwrap(regions_map)
            .expect("Arc still has multiple strong counts")
            .into_inner();

        Ok(Regions(final_map))
    }
}

/**
========================================
SYSTEM API
========================================
*/

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct System {
    id: u32,
    constellation_id: u32,
    position: Point,
    security_status: f32,
    name: String,
}

// This struct directly matches the API response for a single system
#[derive(Deserialize)]
pub struct SystemApiResponse {
    system_id: u32,
    constellation_id: u32,
    position: Point,
    security_status: f32,
    name: String,
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Systems(pub HashMap<u32, System>);

impl Systems {
    pub async fn fetch_all() -> Result<Systems, Box<dyn std::error::Error>> {
        let ids = reqwest::get("https://esi.evetech.net/latest/universe/systems/")
            .await?
            .json::<Vec<u32>>()
            .await?;
        let systems_map = Arc::new(Mutex::new(HashMap::new()));
        let mut handles = Vec::new();

        for id in ids {
            let systems_map = systems_map.clone();
            let handle = tokio::spawn(async move {
                // Consider better error handling than unwrap() in production code
                let info = reqwest::get(format!(
                    "https://esi.evetech.net/latest/universe/systems/{id}" // Corrected endpoint
                ))
                .await
                .expect("Failed to fetch system info") // More specific error message
                .json::<SystemApiResponse>() // Deserialize into SystemApiResponse
                .await
                .expect("Failed to deserialize system info"); // More specific error message

                println!("Reqwested {id}");

                let mut systems_map = systems_map.lock().await;

                // Map SystemApiResponse fields to System fields
                systems_map.insert(
                    info.system_id,
                    System {
                        id: info.system_id,
                        constellation_id: info.constellation_id,
                        position: info.position,
                        security_status: info.security_status,
                        name: info.name,
                    },
                );
            });

            handles.push(handle);
        }

        // Use futures::future::try_join_all to handle errors from spawned tasks
        futures::future::try_join_all(handles)
            .await
            .expect("Error in one of the spawned tasks");

        let final_map = Arc::try_unwrap(systems_map)
            .expect("Arc still has multiple strong counts")
            .into_inner();

        Ok(Systems(final_map))
    }
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Station {
    id: u32,
    system: System,
    name: String,
}
