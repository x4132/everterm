use dashmap::DashMap;
use serde::Deserialize;
use std::error::Error;
use std::fmt::{self, write};
use std::ops::Range;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, OnceCell};

use crate::{ESIClient, esi_url};

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

#[derive(Debug)]
pub struct InvalidIDError {
    value: u32,
    acceptable: Range<u32>,
}

impl fmt::Display for InvalidIDError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Value {} is outside the valid range [{}, {}]",
            self.value, self.acceptable.start, self.acceptable.end
        )
    }
}

impl Error for InvalidIDError {}

/**
========================================
REGION API
========================================
*/

#[derive(Clone, PartialEq, Debug, Eq, Hash, Copy)]
pub struct RegionID {
    value: u32,
}
impl RegionID {
    pub fn get(&self) -> u32 {
        self.value
    }
    pub fn set(&mut self, new_val: u32) {
        self.value = new_val
    }
}

impl<'de> Deserialize<'de> for RegionID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        RegionID::try_from(value).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<u32> for RegionID {
    type Error = InvalidIDError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            10_000_000..20_000_000 => Ok(RegionID { value }),
            _ => Err(InvalidIDError {
                value,
                acceptable: 10_000_000..20_000_000,
            }),
        }
    }
}

/// This struct represents a region in the EvE universe.
#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Region {
    #[serde(rename = "region_id")]
    pub id: RegionID,
    pub name: String,
}

type RegionResult = Result<Region, Box<dyn Error>>;

#[derive(Clone, Debug)]
pub struct Regions {
    pub map: DashMap<RegionID, Region>,
    client: ESIClient,
}

impl Regions {
    pub fn new(client: ESIClient) -> Self {
        Regions {
            map: DashMap::new(),
            client,
        }
    }

    /// Fetches all regions in the universe and returns a Regions object with all regions
    pub async fn fetch_all(client: ESIClient) -> Result<Self, Box<dyn std::error::Error>> {
        let regions = Regions::new(client);
        // WHY - is this really necessary
        let ids: Vec<RegionID> = regions
            .client
            .esi_get("/universe/regions/")
            .await?
            .json::<Vec<u32>>()
            .await?
            .into_iter()
            .map(|v| RegionID::try_from(v))
            .filter(|v| v.is_ok())
            .map(|v| v.unwrap())
            .collect();

        let regions_map: Arc<DashMap<RegionID, Region>> = Arc::new(DashMap::new());
        let mut handles = Vec::new();

        let regions = Arc::new(regions);
        for id in ids {
            let regions_map = regions_map.clone();
            let regions = regions.clone();
            let handle = tokio::spawn(async move {
                let info = regions.get_region(id).await.expect("Failed to load region");

                regions_map.insert(info.id, info);
            });

            handles.push(handle);
        }

        futures::future::try_join_all(handles)
            .await
            .expect("Error in one of the spawned tasks");

        let mut regions = Arc::try_unwrap(regions).expect("Arc still has multiple strong counts");

        regions.map = Arc::try_unwrap(regions_map).expect("Arc still has multiple strong counts");

        Ok(regions)
    }

    /// retrieves a region from an ID.
    pub async fn get_region(&self, id: RegionID) -> RegionResult {
        {
            if let Some(data) = self.map.get(&id) {
                return Ok(data.clone());
            }
        }

        self.fetch_region(id).await
    }

    async fn fetch_region(&self, id: RegionID) -> RegionResult {
        let region: Region;

        {
            // locking the map at ID BECAUSE WHY
            // this feels wrong
            // TODO: research better ways to deal with this mess
            self.map.get(&id);
            region = self
                .client
                .esi_get(&format!("/universe/regions/{}/", id.get()))
                .await?
                .json::<Region>()
                .await?;
        }

        self.map.insert(id, region.clone());

        Ok(region)
    }
}

// /**
// ========================================
// SYSTEM API
// ========================================
// */

// #[derive(Clone, Deserialize, PartialEq, Debug)]
// pub struct System {
//     pub system_id: u32,
//     pub constellation_id: u32,
//     pub position: Point,
//     pub security_status: f32,
//     pub name: String,
// }

// impl System {
//     pub async fn get_system(id: u32) -> SystemResult {
//         let cache = SYSTEMS.get_or_init(systems_cache_init).await;

//         {
//             let cache = cache.lock().await;
//             if let Some(data) = cache.0.get(&id) {
//                 return Ok(data.clone());
//             }
//         }

//         System::fetch_system(id).await
//     }

//     async fn fetch_system(id: u32) -> SystemResult {
//         let system = reqwest::get(esi_url!("/universe/systems/{}", id))
//             .await?
//             .json::<System>()
//             .await?;

//         let mut cache = SYSTEMS.get_or_init(systems_cache_init).await.lock().await;
//         cache.0.insert(system.system_id, system.clone());

//         Ok(system)
//     }
// }

// type SystemResult = Result<System, Box<dyn Error>>;

// #[derive(Clone, Deserialize, PartialEq, Debug)]
// pub struct Systems(pub HashMap<u32, System>);

// impl Systems {
//     pub fn new() -> Self {
//         Systems(HashMap::new())
//     }

//     pub async fn fetch_all() -> Result<Systems, Box<dyn std::error::Error>> {
//         let ids = reqwest::get(esi_url!("/universe/systems/"))
//             .await?
//             .json::<Vec<u32>>()
//             .await?;
//         let systems_map = Arc::new(Mutex::new(HashMap::new()));
//         let mut handles = Vec::new();

//         for id in ids {
//             let systems_map = systems_map.clone();
//             let handle = tokio::spawn(async move {
//                 let info = System::fetch_system(id).await.unwrap();

//                 let mut systems_map = systems_map.lock().await;

//                 systems_map.insert(info.system_id, info);
//             });

//             handles.push(handle);
//         }

//         futures::future::try_join_all(handles)
//             .await
//             .expect("Error in one of the spawned tasks");

//         let final_map = Arc::try_unwrap(systems_map)
//             .expect("Arc still has multiple strong counts")
//             .into_inner();

//         Ok(Systems(final_map))
//     }
// }

// /**
// ========================================
// STATION API
// ========================================
// */

// static STATIONS: OnceCell<Arc<Mutex<HashMap<u64, Station>>>> = OnceCell::const_new();

// type StationResult = Result<Station, Box<dyn Error>>;

// #[derive(Clone, Deserialize, PartialEq, Debug)]
// struct StationApiResponse {
//     station_id: u64,
//     system_id: u32,
//     name: String,
// }

// #[derive(Clone, Deserialize, PartialEq, Debug)]
// pub struct Station {
//     pub id: u64,
//     pub system: System,
//     pub name: String,
// }

// #[derive(Debug, Clone)]
// pub struct StationError(&'static str);
// impl fmt::Display for StationError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", self.0)
//     }
// }
// impl std::error::Error for StationError {}

// impl Station {
//     pub async fn get_station(id: u64) -> StationResult {
//         let cache = STATIONS
//             .get_or_init(async || Arc::new(Mutex::new(HashMap::new())))
//             .await;

//         {
//             let cache = cache.lock().await;
//             if let Some(data) = cache.get(&id) {
//                 return Ok(data.clone());
//             }
//         }

//         Station::fetch_station(id).await
//     }

//     async fn fetch_station(id: u64) -> StationResult {
//         if id >= 60000000 && id <= 64000000 {
//             let mut cache = STATIONS.get().unwrap().lock().await;

//             println!("Fetching station {id}");
//             let resp = reqwest::get(esi_url!("/universe/stations/{}", id)).await?;
//             if resp.error_for_status_ref().is_err() {
//                 return Err(Box::new(resp.error_for_status_ref().err().unwrap()));
//             }
//             let resp = resp.json::<StationApiResponse>().await.unwrap();

//             let system = System::get_system(resp.system_id).await?;

//             let station = Station {
//                 id: resp.station_id,
//                 system,
//                 name: resp.name,
//             };

//             cache.insert(station.id, station.clone());

//             Ok(station)
//         } else {
//             Err(Box::new(StationError("Not a Station")))
//         }
//     }
// }

// /**
// ========================================
// TYPES API
// ========================================
// */

// static TYPES: OnceCell<Arc<Mutex<HashMap<u32, Item>>>> = OnceCell::const_new();

// #[derive(Clone, Deserialize, PartialEq, Debug)]
// pub struct Item {
//     type_id: u32,
//     group_id: u32,
//     icon_id: u32,
//     market_group_id: u32,
//     name: String,
//     description: String,
// }

// type TypeResult = Result<Item, Box<dyn Error>>;
// impl Item {
//     pub async fn get_type(id: u32) -> TypeResult {
//         let cache = TYPES
//             .get_or_init(async || Arc::new(Mutex::new(HashMap::new())))
//             .await;

//         {
//             let cache = cache.lock().await;
//             if let Some(data) = cache.get(&id) {
//                 return Ok(data.clone());
//             }
//         }

//         Item::fetch_type(id).await
//     }

//     async fn fetch_type(id: u32) -> TypeResult {
//         let eve_type = reqwest::get(esi_url!("/universe/types/{}", id))
//             .await?
//             .json::<Item>()
//             .await?;

//         let mut cache = TYPES.get().unwrap().lock().await;
//         cache.insert(eve_type.type_id, eve_type.clone());

//         Ok(eve_type)
//     }
// }
