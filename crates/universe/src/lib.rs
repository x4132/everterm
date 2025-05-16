use std::error::Error;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, OnceCell};

use serde::Deserialize;

mod macros;

/*
ID RANGES
https://developers.eveonline.com/docs/guides/id-ranges/
| From          | To            | Description                                                          |
|---------------|---------------|----------------------------------------------------------------------|
| 0             | 9,999         | System items (including junkyards and other special purpose items)   |
| 500,000       | 599,999       | Factions                                                             |
| 1,000,000     | 1,999,999     | NPC corporations                                                     |
| 3,000,000     | 3,999,999     | NPC characters (agents and NPC corporation CEOs)                     |
| 9,000,000     | 9,999,999     | Universes                                                            |
| 10,000,000    | 19,999,999    | Regions                                                              |
| 10,000,000    | 10,999,999    | New Eden (known space) regions                                       |
| 11,000,000    | 11,999,999    | Wormhole regions                                                     |
| 12,000,000    | 12,999,999    | Abyssal regions                                                      |
| 14,000,000    | 14,999,999    | Void regions                                                         |
| 20,000,000    | 29,999,999    | Constellations                                                       |
| 20,000,000    | 20,999,999    | New Eden (known space) constellations                                |
| 21,000,000    | 21,999,999    | Wormhole constellations                                              |
| 22,000,000    | 22,999,999    | Abyssal constellations                                               |
| 24,000,000    | 24,999,999    | Void constellations                                                  |
| 30,000,000    | 39,999,999    | Solar systems                                                        |
| 30,000,000    | 30,999,999    | New Eden (known space) solar systems                                 |
| 31,000,000    | 31,999,999    | Wormhole solar systems                                               |
| 32,000,000    | 32,999,999    | Abyssal systems                                                      |
| 34,000,000    | 34,999,999    | Void systems                                                         |
| 40,000,000    | 49,999,999    | Celestials (suns, planets, moons, asteroid belts)                    |
| 50,000,000    | 59,999,999    | Stargates                                                            |
| 60,000,000    | 63,999,999    | Stations                                                             |
| 60,000,000    | 60,999,999    | Stations created by CCP                                              |
| 61,000,000    | 63,999,999    | Stations created from outposts                                       |
| 66,000,000    | 67,999,999    | Station folders of corporation offices                               |
| 68,000,000    | 68,999,999    | Station folders for stations created by CCP                          |
| 69,000,000    | 69,999,999    | Station folders for stations created from outposts                   |
| 70,000,000    | 79,999,999    | Asteroids                                                            |
| 80,000,000    | 80,099,999    | Control Bunkers                                                      |
| 81,000,000    | 81,999,999    | WiS Promenades                                                       |
| 82,000,000    | 84,999,999    | Planetary Districts                                                  |
| 90,000,000    | 97,999,999    | EVE characters created between 2010-11-03 and 2016-05-30             |
| 98,000,000    | 98,999,999    | EVE corporations created after 2010-11-03                            |
| 99,000,000    | 99,999,999    | EVE alliances created after 2010-11-03                               |
| 100,000,000   | 2,099,999,999 | EVE characters, corporations and alliances created before 2010-11-03 |
| 2,100,000,000 | 2,111,999,999 | DUST characters, EVE characters created after 2016-05-30             |
| 2,112,000,000 | 2,129,999,999 | EVE characters created after 2016-05-30                              |
*/
#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct RegionId(u32);

impl RegionId {
    pub fn build(id: u32) -> Option<Self> {
        if id >= 10_000_000 && id <= 19_999_999 {
            Some(RegionId(id))
        } else {
            None
        }
    }
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct SystemId(u32);

impl SystemId {
    pub fn build(id: u32) -> Option<Self> {
        // Solar systems range: 30,000,000 - 39,999,999
        if id >= 30_000_000 && id <= 39_999_999 {
            Some(SystemId(id))
        } else {
            None
        }
    }
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct StationId(u32);

impl StationId {
    pub fn build(id: u32) -> Option<Self> {
        // Stations range: 60,000,000 - 63,999,999
        if id >= 60_000_000 && id <= 63_999_999 {
            Some(StationId(id))
        } else {
            None
        }
    }
}

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
    fn from(api_request: RegionAPIRequest) -> Self {
        Region {
            id: api_request.region_id,
            name: api_request.name,
        }
    }

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
    /// Converts a serialized region ESI request into the region struct.
    /// # Examples
    /// ```
    /// use universe::{Region, RegionAPIRequest};
    /// let region = Region::from(RegionAPIRequest {region_id: 10000002, name: String::from("The Forge")});
    /// assert_eq!(region, Region {id: 10000002, name: String::from("The Forge")});
    /// ```
    fn from(api_request: RegionAPIRequest) -> Self {
        Region {
            id: api_request.region_id,
            name: api_request.name,
        }
    }
}

type RegionResult = Result<Region, Box<dyn Error>>;

#[derive(Deserialize)]
pub struct RegionAPIRequest {
    pub region_id: u32,
    pub name: String,
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Regions(pub HashMap<u32, Region>);

impl Regions {
    pub fn new() -> Self {
        Regions(HashMap::new())
    }

    pub async fn fetch_all() -> Result<Regions, Box<dyn std::error::Error>> {
        let ids = reqwest::get("https://esi.evetech.net/latest/universe/regions/")
            .await?
            .json::<Vec<u32>>()
            .await?;
        let regions_map = Arc::new(Mutex::new(HashMap::new()));
        let mut handles = Vec::new();

        for id in ids {
            let regions_map = regions_map.clone();
            let handle = tokio::spawn(async move {
                let info = reqwest::get(format!(
                    "https://esi.evetech.net/latest/universe/regions/{id}"
                ))
                .await
                .expect("Failed to fetch region info") // More specific error message
                .json::<RegionAPIRequest>()
                .await
                .expect("Failed to deserialize region info"); // More specific error message

                let mut regions_map = regions_map.lock().await;
                regions_map.insert(
                    info.region_id,
                    Region {
                        id: info.region_id,
                        name: info.name,
                    },
                );
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
