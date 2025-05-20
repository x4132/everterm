use std::error::Error;
use std::fmt;
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
    pub region_id: u32,
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
            .json::<Region>()
            .await?;

        let mut cache = REGIONS.get_or_init(regions_cache_init).await.lock().await;
        cache.0.insert(id, region.clone());

        Ok(region)
    }
}

type RegionResult = Result<Region, Box<dyn Error>>;

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
                regions_map.insert(info.region_id, info);
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

static SYSTEMS: OnceCell<Arc<Mutex<Systems>>> = OnceCell::const_new();

async fn systems_cache_init() -> Arc<Mutex<Systems>> {
    Arc::new(Mutex::new(Systems::new()))
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct System {
    pub system_id: u32,
    pub constellation_id: u32,
    pub position: Point,
    pub security_status: f32,
    pub name: String,
}

impl System {
    pub async fn get_system(id: u32) -> SystemResult {
        let cache = SYSTEMS.get_or_init(systems_cache_init).await;

        {
            let cache = cache.lock().await;
            if let Some(data) = cache.0.get(&id) {
                return Ok(data.clone());
            }
        }

        System::fetch_system(id).await
    }

    async fn fetch_system(id: u32) -> SystemResult {
        let system = reqwest::get(esi!("/universe/systems/{}", id))
            .await?
            .json::<System>()
            .await?;

        let mut cache = SYSTEMS.get_or_init(systems_cache_init).await.lock().await;
        cache.0.insert(system.system_id, system.clone());

        Ok(system)
    }
}

type SystemResult = Result<System, Box<dyn Error>>;

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Systems(pub HashMap<u32, System>);

impl Systems {
    pub fn new() -> Self {
        Systems(HashMap::new())
    }

    pub async fn fetch_all() -> Result<Systems, Box<dyn std::error::Error>> {
        let ids = reqwest::get(esi!("/universe/systems/"))
            .await?
            .json::<Vec<u32>>()
            .await?;
        let systems_map = Arc::new(Mutex::new(HashMap::new()));
        let mut handles = Vec::new();

        for id in ids {
            let systems_map = systems_map.clone();
            let handle = tokio::spawn(async move {
                let info = System::fetch_system(id).await.unwrap();

                let mut systems_map = systems_map.lock().await;

                systems_map.insert(info.system_id, info);
            });

            handles.push(handle);
        }

        futures::future::try_join_all(handles)
            .await
            .expect("Error in one of the spawned tasks");

        let final_map = Arc::try_unwrap(systems_map)
            .expect("Arc still has multiple strong counts")
            .into_inner();

        Ok(Systems(final_map))
    }
}

/**
========================================
STATION API
========================================
*/

static STATIONS: OnceCell<Arc<Mutex<HashMap<u64, Station>>>> = OnceCell::const_new();

type StationResult = Result<Station, Box<dyn Error>>;

#[derive(Clone, Deserialize, PartialEq, Debug)]
struct StationApiResponse {
    station_id: u64,
    system_id: u32,
    name: String,
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Station {
    pub id: u64,
    pub system: System,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct StationError(&'static str);
impl fmt::Display for StationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for StationError {}

impl Station {
    pub async fn get_station(id: u64) -> StationResult {
        let cache = STATIONS
            .get_or_init(async || Arc::new(Mutex::new(HashMap::new())))
            .await;

        {
            let cache = cache.lock().await;
            if let Some(data) = cache.get(&id) {
                return Ok(data.clone());
            }
        }

        Station::fetch_station(id).await
    }

    async fn fetch_station(id: u64) -> StationResult {
        if id >= 60000000 && id <= 64000000 {
            let mut cache = STATIONS.get().unwrap().lock().await;

            println!("Fetching station {id}");
            let resp = reqwest::get(esi!("/universe/stations/{}", id)).await?;
            if resp.error_for_status_ref().is_err() {
                return Err(Box::new(resp.error_for_status_ref().err().unwrap()));
            }
            let resp = resp.json::<StationApiResponse>().await.unwrap();

            let system = System::get_system(resp.system_id).await?;

            let station = Station {
                id: resp.station_id,
                system,
                name: resp.name,
            };

            cache.insert(station.id, station.clone());

            Ok(station)
        } else {
            Err(Box::new(StationError("Not a Station")))
        }
    }
}

/**
========================================
TYPES API
========================================
*/

static TYPES: OnceCell<Arc<Mutex<HashMap<u32, Item>>>> = OnceCell::const_new();

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Item {
    type_id: u32,
    group_id: u32,
    icon_id: u32,
    market_group_id: u32,
    name: String,
    description: String,
}

type TypeResult = Result<Item, Box<dyn Error>>;
impl Item {
    pub async fn get_type(id: u32) -> TypeResult {
        let cache = TYPES
            .get_or_init(async || Arc::new(Mutex::new(HashMap::new())))
            .await;

        {
            let cache = cache.lock().await;
            if let Some(data) = cache.get(&id) {
                return Ok(data.clone());
            }
        }

        Item::fetch_type(id).await
    }

    async fn fetch_type(id: u32) -> TypeResult {
        let eve_type = reqwest::get(esi!("/universe/types/{}", id))
            .await?
            .json::<Item>()
            .await?;

        let mut cache = TYPES.get().unwrap().lock().await;
        cache.insert(eve_type.type_id, eve_type.clone());

        Ok(eve_type)
    }
}
