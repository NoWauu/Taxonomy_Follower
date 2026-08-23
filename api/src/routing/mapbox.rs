//! [Mapbox Directions API] client.
//!
//! One GET per route: the waypoints go into the path as `lon,lat` pairs
//! separated by semicolons, and the answer carries the whole route plus one leg
//! per pair of consecutive waypoints. Geometries are asked for as `polyline6`,
//! which is an order of magnitude smaller on the wire than GeoJSON and is what
//! every Mapbox front end decoder expects.
//!
//! [Mapbox Directions API]: https://docs.mapbox.com/api/navigation/directions/

use std::time::Duration;

use anyhow::Context;
use async_trait::async_trait;
use serde::Deserialize;

use crate::config::MapboxConfig;
use crate::error::{ApiError, ApiResult};

use super::{Route, RouteLeg, RouteRequest, RoutingProvider};

pub struct MapboxRoutingProvider {
    http: reqwest::Client,
    base_url: String,
    access_token: String,
}

impl MapboxRoutingProvider {
    pub fn new(config: &MapboxConfig, timeout: Duration) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent(concat!("taxonomy-follower/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("failed to build the Mapbox HTTP client")?;

        Ok(Self {
            http,
            base_url: config.base_url.clone(),
            access_token: config.access_token.clone(),
        })
    }
}

#[async_trait]
impl RoutingProvider for MapboxRoutingProvider {
    async fn route(&self, request: RouteRequest) -> ApiResult<Route> {
        request.validate()?;

        let coordinates = request
            .waypoints
            .iter()
            .map(|point| format!("{},{}", point.longitude, point.latitude))
            .collect::<Vec<_>>()
            .join(";");

        let url = format!(
            "{}/directions/v5/mapbox/{}/{coordinates}",
            self.base_url,
            request.profile.as_str(),
        );

        let response = self
            .http
            .get(&url)
            .query(&[
                ("access_token", self.access_token.as_str()),
                ("geometries", "polyline6"),
                ("overview", "full"),
                ("steps", "false"),
                ("alternatives", "false"),
            ])
            .send()
            .await
            .context("failed to reach the Mapbox Directions API")?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("failed to read the Mapbox Directions response")?;

        // Mapbox answers 200 for a request it understood but could not route,
        // and 4xx with the same JSON shape otherwise, so the body is parsed
        // before the status is judged.
        let payload: DirectionsResponse = serde_json::from_str(&body).with_context(|| {
            format!("unexpected Mapbox Directions response (HTTP {status}): {body}")
        })?;

        // A 401 or a 5xx from the edge carries no `code` at all, hence the
        // fallback: anything unrecognised lands in the catch-all arm below.
        match payload.code.as_deref().unwrap_or("Unknown") {
            "Ok" => {}
            // The waypoints are fine, there is simply no road connecting them.
            "NoRoute" | "NoSegment" => {
                return Err(ApiError::BadRequest(
                    "no route connects these locations".to_string(),
                ));
            }
            "InvalidInput" => {
                return Err(ApiError::BadRequest(
                    payload
                        .message
                        .unwrap_or_else(|| "Mapbox rejected these waypoints".to_string()),
                ));
            }
            other => {
                // Bad token, exhausted quota, unknown profile: our problem, not
                // the caller's.
                return Err(ApiError::Internal(anyhow::anyhow!(
                    "Mapbox Directions failed with `{other}` (HTTP {status}): {}",
                    payload.message.unwrap_or_else(|| body.clone())
                )));
            }
        }

        let route = payload
            .routes
            .into_iter()
            .next()
            .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Mapbox returned no route")))?;

        Ok(Route {
            distance_meters: route.distance,
            duration_seconds: route.duration,
            legs: route
                .legs
                .into_iter()
                .map(|leg| RouteLeg {
                    distance_meters: leg.distance,
                    duration_seconds: leg.duration,
                })
                .collect(),
            geometry: route.geometry,
            estimated: false,
        })
    }
}

#[derive(Debug, Deserialize)]
struct DirectionsResponse {
    #[serde(default)]
    code: Option<String>,
    message: Option<String>,
    #[serde(default)]
    routes: Vec<DirectionsRoute>,
}

#[derive(Debug, Deserialize)]
struct DirectionsRoute {
    /// Metres.
    distance: f64,
    /// Seconds.
    duration: f64,
    /// polyline6-encoded, since that is what the request asks for.
    geometry: Option<String>,
    #[serde(default)]
    legs: Vec<DirectionsLeg>,
}

#[derive(Debug, Deserialize)]
struct DirectionsLeg {
    distance: f64,
    duration: f64,
}
