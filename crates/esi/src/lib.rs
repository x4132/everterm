use base64::prelude::*;
use http_cache_reqwest::{
    CACacheManager, Cache, CacheMode, CacheOptions, HttpCache, HttpCacheOptions,
};
use macros::ESI_URL;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Response, header::USER_AGENT};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Error as MiddlewareError};
use std::clone;
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
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
    auth_tok: Option<String>,
}

impl ESIClient {
    pub fn new(component_name: &str, platform_name: &str, max_sem: usize) -> Self {
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
                mode: CacheMode::Default,
                manager: CACacheManager::default(),
                options: HttpCacheOptions {
                    cache_key: None,
                    cache_mode_fn: None,
                    cache_options: Some(CacheOptions {
                        shared: true,
                        cache_heuristic: 0.01,
                        ignore_cargo_cult: false,
                        immutable_min_time_to_live: Duration::from_secs(24 * 3600),
                    }),
                    cache_bust: None,
                    cache_status_headers: true,
                },
            }))
            .build(), // cursed
            component_name: String::from(component_name),
            platform_name: String::from(platform_name),
            connect_pool: Arc::new(Semaphore::new(max_sem)),
            auth_tok: None,
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

        let mut req = self.client.get([ESI_URL, url].join(""))
            .header(USER_AGENT, format!("{}; component of EvERTerm/0.0.1 (0@x4132.dev; +https://github.com/x4132/everterm; discord:msvcredist2022; eve:Charles Helugo) on {}", self.component_name, self.platform_name));

        if self.auth_tok_valid().await {
            req = req.header(AUTHORIZATION, format!("Bearer {}", self.auth_tok.clone().unwrap_or(String::from("NOACL"))));
        }

        // send first request via middleware and map all errors into MiddlewareError
        let mut result: Result<Response, MiddlewareError> = {
            let first = req.try_clone().unwrap().send().await;
            first.map_err(|e| e.into())
        };

        // try again once if it's just a regular http error
        // TODO: Evaluate if this is really necessary?
        // NOTE: Do i need another permit?
        if result.is_err() {
            println!(
                "ESI Client: Needed to resend request! {:?}",
                result.as_ref().err().unwrap()
            );
            *self.errors.lock().await -= 1;
            // retry and convert reqwest::Error into our MiddlewareError
            result = req.send().await.map_err(|e| e.into());
        }

        if result.is_err() {
            return Err(result.err().unwrap());
        }

        let result = result.unwrap();

        drop(permit);

        // unify status errors into MiddlewareError via .into()
        match result.status().as_u16() {
            200 => Ok(result),
            420 => {
                self.await_esi_timeout().await;

                Err(result.error_for_status().unwrap_err().into())
            }
            400..=499 => {
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

                let err = result.error_for_status().unwrap_err();

                Err(err.into())
            }
            500..=599 => {
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

    /// check if auth token is valid
    pub async fn auth_tok_valid(&self) -> bool {
        match self.auth_tok.clone() {
            Some(tok) => {
                // Parse JWT token to check expiry
                let parts: Vec<&str> = tok.split('.').collect();
                if parts.len() != 3 {
                    return false;
                }

                // Decode the payload (second part of JWT)
                let payload_b64 = parts[1];
                // Add padding if needed for base64 decoding
                let padded_payload = match payload_b64.len() % 4 {
                    0 => payload_b64.to_string(),
                    n => format!("{}{}", payload_b64, "=".repeat(4 - n)),
                };

                match BASE64_STANDARD.decode(padded_payload.as_bytes()) {
                    Ok(decoded) => {
                        match serde_json::from_slice::<serde_json::Value>(&decoded) {
                            Ok(payload) => {
                                if let Some(exp) = payload.get("exp").and_then(|v| v.as_u64()) {
                                    let current_time = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs();

                                    // Token is valid if current time is less than expiry time
                                    current_time < exp
                                } else {
                                    false // No expiry claim found
                                }
                            }
                            Err(_) => false, // Failed to parse JSON
                        }
                    }
                    Err(_) => false, // Failed to decode base64
                }
            }
            None => false,
        }
    }

    pub async fn load_auth_tok(
        &mut self,
        refresh_tok: String,
        client_id: String,
        client_secret: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let auth_str =
            BASE64_STANDARD.encode(format!("{}:{}", client_id, client_secret).as_bytes());

        let response = self
            .client
            .post("https://login.eveonline.com/v2/oauth/token")
            .header("Authorization", format!("Basic {}", auth_str))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", &refresh_tok),
            ])
            .send()
            .await?;

        let token_response: serde_json::Value = response.json().await?;
        let access_token = token_response["access_token"]
            .as_str()
            .ok_or("Missing access_token in response")?
            .to_string();

        self.auth_tok = Some(access_token);

        Ok(())
    }

    async fn await_esi_timeout(&self) {
        let timeout = self.error_timeout.lock().await;

        sleep(Duration::from_secs((*timeout).into())).await
    }
}
