//! Straight-line fallback used when no Mapbox token is configured.
//!
//! It measures great-circle distances between consecutive waypoints and divides
//! them by an average speed. Roads are longer and slower than that, so every
//! route it returns is flagged `estimated`; it exists to keep local development
//! and tests running without an account, not to be shown to a user.

use async_trait::async_trait;

use crate::error::ApiResult;

use super::{Coordinates, Route, RouteLeg, RouteRequest, RoutingProvider};

/// Mean Earth radius in metres, as used by the haversine formula.
const EARTH_RADIUS_METERS: f64 = 6_371_008.8;

pub struct HaversineRoutingProvider;

impl HaversineRoutingProvider {
    pub fn new() -> Self {
        tracing::warn!(
            "MAPBOX_ACCESS_TOKEN is not set: routes are straight-line estimates, not real itineraries"
        );
        Self
    }
}

impl Default for HaversineRoutingProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RoutingProvider for HaversineRoutingProvider {
    async fn route(&self, request: RouteRequest) -> ApiResult<Route> {
        request.validate()?;

        let speed = request.profile.average_speed_mps();

        let legs: Vec<RouteLeg> = request
            .waypoints
            .windows(2)
            .map(|pair| {
                let distance_meters = haversine_meters(pair[0], pair[1]);
                RouteLeg {
                    distance_meters,
                    duration_seconds: distance_meters / speed,
                }
            })
            .collect();

        Ok(Route {
            distance_meters: legs.iter().map(|leg| leg.distance_meters).sum(),
            duration_seconds: legs.iter().map(|leg| leg.duration_seconds).sum(),
            legs,
            geometry: None,
            estimated: true,
        })
    }
}

fn haversine_meters(from: Coordinates, to: Coordinates) -> f64 {
    let (from_lat, to_lat) = (from.latitude.to_radians(), to.latitude.to_radians());
    let delta_lat = to_lat - from_lat;
    let delta_lon = (to.longitude - from.longitude).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + from_lat.cos() * to_lat.cos() * (delta_lon / 2.0).sin().powi(2);

    2.0 * EARTH_RADIUS_METERS * a.sqrt().asin()
}
