use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use macros::ESI_URL;
use reqwest::{Response, header::USER_AGENT};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Error as MiddlewareError};
use std::{sync::Arc, time::Duration};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::sleep;

mod macros;
pub mod market;
pub mod universe;

#[derive(Clone, Debug)]
pub struct ESIClient {
    errors: Arc<Mutex<u32>>,
    error_timeout: Arc<Mutex<u32>>,
    client: ClientWithMiddleware,
    component_name: String,
    platform_name: String,
    connect_pool: Arc<Semaphore>, // we need this to not run out of open fd's
}

impl ESIClient {
    pub fn new(component_name: &str, platform_name: &str) -> Self {
        ESIClient {
            errors: Arc::new(Mutex::new(100)),
            error_timeout: Arc::new(Mutex::new(0)),
            client: ClientBuilder::new(
                reqwest::Client::builder()
                    .pool_max_idle_per_host(32)
                    .pool_idle_timeout(Duration::from_secs(15))
                    .build()
                    .unwrap(),
            )
            .with(Cache(HttpCache {
                mode: CacheMode::ForceCache,
                manager: CACacheManager::default(),
                options: HttpCacheOptions::default(),
            }))
            .build(), // cursed
            component_name: String::from(component_name),
            platform_name: String::from(platform_name),
            // TODO: bump this up on prod when i jack up the ulimit (fd limit)
            // also make this function jack up the ulimit for this app
            connect_pool: Arc::new(Semaphore::new(32)),
        }
    }

    pub async fn esi_get(&self, url: &str) -> Result<Response, MiddlewareError> {
        let permit = self.connect_pool.acquire().await.unwrap();

        {
            // this blocks everything cuz it locks and doesnt unlock until it waits out the timer
            let errors = self.errors.lock().await;
            if *errors <= 10 {
                self.await_esi_timeout().await;
            }
        }

        let req = self.client.get([ESI_URL, url].join(""))
            .header(USER_AGENT, format!("{}; component of EvERTerm/0.0.1 (0@x4132.dev; +https://github.com/x4132/everterm; discord:msvcredist2022; eve:Charles Helugo) on {}", self.component_name, self.platform_name));

        // send first request via middleware and map all errors into MiddlewareError
        let mut result: Result<Response, MiddlewareError> = {
            let first = req.try_clone().unwrap().send().await;
            first.map_err(|e| e.into())
        };

        // try again once if it's just a regular http error
        // TODO: Evaluate if this is really necessary?
        // NOTE: Do i need another permit?
        if result.is_err() {
            eprintln!(
                "ESI Client: Needed to resend request! {:?}",
                result.as_ref().err().unwrap()
            );
            // retry and convert reqwest::Error into our MiddlewareError
            result = req.send().await.map_err(|e| e.into());
        }

        if result.is_err() {
            return Err(result.err().unwrap());
        }

        let result = result.unwrap();

        // TODO: maybe add better error handling here, but this is probably fine??
        (*self.errors.lock().await) = result
            .headers()
            .get("x-esi-error-limit-remain")
            .expect("No ESI Error limit found???")
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        (*self.error_timeout.lock().await) = result
            .headers()
            .get("x-esi-error-limit-reset")
            .expect("No ESI Reset Timer found???")
            .to_str()
            .unwrap()
            .parse()
            .unwrap();

        drop(permit);
        println!("Returned semaphore");

        // unify status errors into MiddlewareError via .into()
        match result.status().as_u16() {
            200 => Ok(result),
            400..=599 => {
                let err = result.error_for_status().unwrap_err();
                Err(err.into())
            }
            _ => {
                eprintln!(
                    "ESI Client: Unknown error code detected! code: {}",
                    result.status()
                );
                let err = result.error_for_status().unwrap_err();
                Err(err.into())
            }
        }
    }

    async fn await_esi_timeout(&self) {
        let timeout = self.error_timeout.lock().await;

        sleep(Duration::from_secs((*timeout).into())).await
    }
}
