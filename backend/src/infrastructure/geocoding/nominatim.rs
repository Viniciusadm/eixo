use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant as TokioInstant};

use crate::application::timeline::PlaceNameResolver;
use crate::domain::Coordinate;
use crate::errors::AppError;

const MIN_INTERVAL: Duration = Duration::from_millis(1100);

pub struct NominatimResolver {
    client: reqwest::Client,
    base_url: String,
    last_request: Mutex<Option<TokioInstant>>,
}

impl NominatimResolver {
    pub fn new(base_url: impl Into<String>, user_agent: &str) -> Result<Self, AppError> {
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|error| AppError::Configuration(error.to_string()))?;

        Ok(Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            last_request: Mutex::new(None),
        })
    }

    async fn throttle(&self) {
        let mut last_request = self.last_request.lock().await;
        if let Some(previous) = *last_request {
            let elapsed = previous.elapsed();
            if elapsed < MIN_INTERVAL {
                sleep(MIN_INTERVAL - elapsed).await;
            }
        }
        *last_request = Some(TokioInstant::now());
    }
}

#[async_trait]
impl PlaceNameResolver for NominatimResolver {
    async fn resolve(&self, coordinate: Coordinate) -> Result<Option<String>, AppError> {
        self.throttle().await;

        let response = self
            .client
            .get(format!("{}/reverse", self.base_url))
            .query(&[
                ("format", "jsonv2"),
                ("zoom", "18"),
                ("addressdetails", "1"),
                ("lat", &coordinate.latitude().to_string()),
                ("lon", &coordinate.longitude().to_string()),
            ])
            .send()
            .await
            .map_err(|error| AppError::Internal(format!("nominatim request failed: {error}")))?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!(
                "nominatim answered {}",
                response.status()
            )));
        }

        let payload: ReverseResponse = response
            .json()
            .await
            .map_err(|error| AppError::Internal(format!("nominatim payload: {error}")))?;

        Ok(short_label(&payload))
    }
}

fn short_label(payload: &ReverseResponse) -> Option<String> {
    let address = payload.address.as_ref();
    let specific = payload
        .name
        .clone()
        .or_else(|| address.and_then(|address| address.amenity.clone()))
        .or_else(|| address.and_then(|address| address.shop.clone()))
        .or_else(|| address.and_then(|address| address.road.clone()))
        .filter(|value| !value.trim().is_empty());

    let mut areas: Vec<String> = address
        .into_iter()
        .flat_map(|address| {
            [
                address.neighbourhood.clone(),
                address.suburb.clone(),
                address.city.clone(),
                address.town.clone(),
                address.village.clone(),
                address.municipality.clone(),
            ]
        })
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .collect();
    areas.dedup();

    let parts: Vec<String> = match specific {
        Some(specific) => {
            let area = areas.into_iter().find(|area| *area != specific);
            std::iter::once(specific).chain(area).collect()
        }
        None => areas.into_iter().take(2).collect(),
    };

    if parts.is_empty() {
        return payload
            .display_name
            .clone()
            .filter(|value| !value.trim().is_empty());
    }
    Some(parts.join(", "))
}

#[derive(Debug, Deserialize)]
struct ReverseResponse {
    name: Option<String>,
    display_name: Option<String>,
    address: Option<Address>,
}

#[derive(Debug, Deserialize)]
struct Address {
    amenity: Option<String>,
    shop: Option<String>,
    road: Option<String>,
    neighbourhood: Option<String>,
    suburb: Option<String>,
    city: Option<String>,
    town: Option<String>,
    village: Option<String>,
    municipality: Option<String>,
}
