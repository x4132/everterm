use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde::Deserialize;

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Point {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Region {
    pub id: u32,
    pub name: String,
}

#[derive(Deserialize)]
struct RegionRequest {
    region_id: u32,
    name: String,
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Regions(pub HashMap<u32, Region>);
impl Regions {
    pub async fn fetch() -> Result<Regions, Box<dyn std::error::Error>> {
        let ids = reqwest::get("https://esi.evetech.net/latest/universe/regions/")
            .await?
            .json::<Vec<u32>>()
            .await?;
        let regions_map = Arc::new(Mutex::new(HashMap::new()));
        let mut handles = Vec::new();

        for id in ids {
            let regions_map = regions_map.clone();
            let handle = tokio::spawn(async move {
                // Consider better error handling than unwrap() in production code
                let info = reqwest::get(format!(
                    "https://esi.evetech.net/latest/universe/regions/{id}"
                ))
                .await
                .expect("Failed to fetch region info") // More specific error message
                .json::<RegionRequest>()
                .await
                .expect("Failed to deserialize region info"); // More specific error message

                let mut regions_map = regions_map.lock().expect("Failed to lock regions map");
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

        // Use futures::future::try_join_all to handle errors from spawned tasks
        futures::future::try_join_all(handles)
            .await
            .expect("Error in one of the spawned tasks");

        let final_map = Arc::try_unwrap(regions_map)
            .expect("Arc still has multiple strong counts")
            .into_inner()
            .expect("Mutex poisoned");

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
    pub async fn fetch() -> Result<Systems, Box<dyn std::error::Error>> {
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

                let mut systems_map = systems_map.lock().expect("Failed to lock systems map");

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
            .into_inner()
            .expect("Mutex poisoned");

        Ok(Systems(final_map))
    }
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Station {
    id: u32,
    system: System,
    name: String,
}