use dashmap::DashMap;
use serde::Deserialize;
use std::error::Error;
use std::fmt::{self};
use std::ops::Range;
use std::sync::Arc;

use crate::ESIClient;

/// This struct represents a geospatial point in the EvE universe.
/// i have no idea what that means
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
    value: u64,
    acceptable: Range<u64>,
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
                value: value.into(),
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
    client: Arc<ESIClient>,
}

impl Regions {
    pub fn new(client: Arc<ESIClient>) -> Self {
        Regions {
            map: DashMap::new(),
            client,
        }
    }

    /// Fetches all regions in the universe and returns a Regions object with all regions
    pub async fn get_all(client: Arc<ESIClient>) -> Result<Self, Box<dyn std::error::Error>> {
        let regions = Regions::new(client);
        // WHY - is this really necessary
        let ids: Vec<RegionID> = regions
            .client
            .esi_get("/universe/regions/")
            .await?
            .json::<Vec<RegionID>>()
            .await?;

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

/**
========================================
SYSTEM API
========================================
*/

#[derive(Clone, PartialEq, Debug, Eq, Hash, Copy)]
pub struct SystemID {
    value: u32,
}
impl SystemID {
    pub fn get(&self) -> u32 {
        self.value
    }
    pub fn set(&mut self, new_val: u32) {
        self.value = new_val
    }
}

impl<'de> Deserialize<'de> for SystemID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        SystemID::try_from(value).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<u32> for SystemID {
    type Error = InvalidIDError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            30_000_000..40_000_000 => Ok(SystemID { value }),
            _ => Err(InvalidIDError {
                value: value.into(),
                acceptable: 30_000_000..40_000_000,
            }),
        }
    }
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct System {
    #[serde(rename = "system_id")]
    pub id: SystemID,
    pub constellation_id: ConstellationID,
    pub position: Point,
    pub security_status: f32,
    pub name: String,
}

type SystemResult = Result<System, Box<dyn Error>>;

#[derive(Clone, Debug)]
pub struct Systems {
    pub map: DashMap<SystemID, System>,
    client: Arc<ESIClient>,
}

impl Systems {
    pub fn new(client: Arc<ESIClient>) -> Self {
        Systems {
            map: DashMap::new(),
            client,
        }
    }

    /// Fetches all regions in the universe and returns a Regions object with all regions
    pub async fn get_all(client: Arc<ESIClient>) -> Result<Self, Box<dyn std::error::Error>> {
        let systems = Systems::new(client);
        // WHY - is this really necessary
        let ids: Vec<SystemID> = systems
            .client
            .esi_get("/universe/systems/")
            .await?
            .json::<Vec<SystemID>>()
            .await?;

        let systems_map: Arc<DashMap<SystemID, System>> = Arc::new(DashMap::new());
        let mut handles = Vec::new();

        let systems = Arc::new(systems);
        for id in ids {
            let systems_map = systems_map.clone();
            let systems = systems.clone();
            let handle = tokio::spawn(async move {
                let info = systems.get_system(id).await.expect("Failed to load system");

                systems_map.insert(info.id, info);
            });

            handles.push(handle);
        }

        futures::future::try_join_all(handles)
            .await
            .expect("Error in one of the spawned tasks");

        let mut systems = Arc::try_unwrap(systems).expect("Arc still has multiple strong counts");

        systems.map = Arc::try_unwrap(systems_map).expect("Arc still has multiple strong counts");

        Ok(systems)
    }

    pub async fn get_system(&self, id: SystemID) -> SystemResult {
        {
            if let Some(data) = self.map.get(&id) {
                return Ok(data.clone());
            }
        }

        self.fetch_system(id).await
    }

    async fn fetch_system(&self, id: SystemID) -> SystemResult {
        let system: System;

        {
            self.map.get(&id);
            system = self
                .client
                .esi_get(&format!("/universe/systems/{}/", id.get()))
                .await?
                .json::<System>()
                .await?;
        }

        self.map.insert(id, system.clone());

        Ok(system)
    }
}

// ========================================
// CONSTELLATION API
// ========================================

#[derive(Clone, PartialEq, Debug, Eq, Hash, Copy)]
pub struct ConstellationID {
    value: u32,
}
impl ConstellationID {
    pub fn get(&self) -> u32 {
        self.value
    }
    pub fn set(&mut self, new_val: u32) {
        self.value = new_val
    }
}

impl<'de> Deserialize<'de> for ConstellationID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        ConstellationID::try_from(value).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<u32> for ConstellationID {
    type Error = InvalidIDError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            20_000_000..30_000_000 => Ok(ConstellationID { value }),
            _ => Err(InvalidIDError {
                value: value.into(),
                acceptable: 20_000_000..30_000_000,
            }),
        }
    }
}

// ========================================
// STATION API
// ========================================

// TODO: structure API
#[derive(Clone, PartialEq, Debug, Eq, Hash, Copy)]
pub struct StationID {
    value: u64,
}
impl StationID {
    pub fn get(&self) -> u64 {
        self.value
    }
    pub fn set(&mut self, new_val: u64) {
        self.value = new_val
    }
}

impl<'de> Deserialize<'de> for StationID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        StationID::try_from(value).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<u64> for StationID {
    type Error = InvalidIDError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            60_000_000..64_000_000 => Ok(StationID { value }),
            _ => Err(InvalidIDError {
                value,
                acceptable: 60_000_000..64_000_000,
            }),
        }
    }
}

type StationResult = Result<Station, Box<dyn Error>>;

#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Station {
    pub id: StationID,
    pub system_id: SystemID,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct Stations {
    pub map: DashMap<StationID, Station>,
    client: Arc<ESIClient>,
}

impl Stations {
    pub fn new(client: Arc<ESIClient>) -> Self {
        Stations {
            map: DashMap::new(),
            client,
        }
    }

    pub async fn get_station(&self, id: StationID) -> StationResult {
        {
            if let Some(data) = self.map.get(&id) {
                return Ok(data.clone());
            }
        }

        self.fetch_station(id).await
    }

    async fn fetch_station(&self, id: StationID) -> StationResult {
        let system: Station;

        {
            self.map.get(&id);
            system = self
                .client
                .esi_get(&format!("/universe/stations/{}/", id.get()))
                .await?
                .json::<Station>()
                .await?;
        }

        self.map.insert(id, system.clone());

        Ok(system)
    }
}

// ========================================
// TYPES API
// ========================================
// types are just u32s, so there isn't a dedicated struct for them

#[derive(Debug)]
pub struct NonMarketableTypeError(u32);
impl fmt::Display for NonMarketableTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Type {} is not marketable", self.0)
    }
}
impl Error for NonMarketableTypeError {}

/// A marketable item. For non-marketable items, see [`ItemRaw`].
#[derive(Clone, Deserialize, PartialEq, Debug)]
pub struct Item {
    id: u32,
    group_id: u32,
    icon_id: u32,
    market_group_id: u32,
    name: String,
    description: String,
}

impl TryFrom<ItemRaw> for Item {
    type Error = NonMarketableTypeError;

    fn try_from(value: ItemRaw) -> Result<Self, Self::Error> {
        match value.market_group_id {
            Some(market_group_id) => Ok(Item {
                id: value.type_id,
                group_id: value.group_id,
                icon_id: value.icon_id,
                market_group_id,
                name: value.name,
                description: value.description,
            }),
            None => Err(NonMarketableTypeError(value.type_id)),
        }
    }
}

/// An optionally marketable item.
#[derive(Deserialize)]
pub struct ItemRaw {
    type_id: u32,
    group_id: u32,
    icon_id: u32,
    market_group_id: Option<u32>,
    name: String,
    description: String,
}

type ItemResult = Result<Item, Box<dyn Error>>;

pub struct Items {
    pub map: DashMap<u32, Item>,
    client: Arc<ESIClient>,
}

impl Items {
    pub fn new(client: Arc<ESIClient>) -> Self {
        Items {
            map: DashMap::new(),
            client,
        }
    }

    /// gets a marketable item from an item id
    pub async fn get_item(&self, id: u32) -> ItemResult {
        {
            if let Some(data) = self.map.get(&id) {
                return Ok(data.clone());
            }
        }

        Ok(Item::try_from(self.fetch_item_raw(id).await?)?)
    }

    pub async fn fetch_item_raw(&self, id: u32) -> Result<ItemRaw, Box<dyn Error>> {
        let raw: ItemRaw = self
            .client
            .esi_get(&format!("/universe/types/{id}/"))
            .await?
            .json::<ItemRaw>()
            .await?;

        Ok(raw)
    }
}
