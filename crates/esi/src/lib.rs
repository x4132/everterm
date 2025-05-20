use macros::ESI_URL;
use reqwest::{Error, Response, header::USER_AGENT};
use std::{
    sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;

use tokio::time::sleep;

mod macros;
pub mod market;
pub mod universe;

pub struct ESIClient {
    errors: Arc<Mutex<u32>>,
    error_timeout: Arc<Mutex<u32>>,
    client: reqwest::Client,
    component_name: String,
    platform_name: String,
}

impl ESIClient {
    pub fn new(component_name: &str, platform_name: &str) -> Self {
        ESIClient {
            errors: Arc::new(Mutex::new(false)),
            error_timeout: Arc::new(Mutex::new(0)),
            client: reqwest::Client::new(),
            component_name: String::from(component_name),
            platform_name: String::from(platform_name),
        }
    }

    pub async fn esi_get(&self, url: &str) -> Result<Response, Error> {
        {
            let errors = self.errors.lock().await;
            if *errors <= 10 {
                self.resolve_esi_errors().await;
            }
        }

        let req = self.client.get([ESI_URL, url].join(""))
            .header(USER_AGENT, format!("{}; component of EvERTerm/0.0.1 (0@x4132.dev; +https://github.com/x4132/everterm; discord:msvcredist2022; eve:Charles Helugo) on {}", self.component_name, self.platform_name));

        let mut result = req.try_clone().unwrap().send().await;

        // try again once if it's just a regular http error
        // TODO: Evaluate if this is really necessary?
        if result.is_err() {
            eprintln!(
                "ESI Client: Needed to resend request! {:?}",
                result.err().unwrap()
            );
            result = req.send().await;
        }

        if result.is_err() {
            return Err(result.err().unwrap());
        }

        let result = result.unwrap();

        // TODO: maybe add better error handling here, but this is probably fine???
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

        match result.status().as_u16() {
            200 => Ok(result),
            400..599 => Err(result.error_for_status().err().unwrap()),
            _ => {
                eprintln!(
                    "ESI Client: Unknown error code detected! code: {}",
                    result.status()
                );
                Err(result.error_for_status().err().unwrap())
            }
        }
    }

    async fn resolve_esi_errors(&self) {
        let timeout = self.error_timeout.lock().await;

        sleep(Duration::from_secs((*timeout).into())).await
    }
}
