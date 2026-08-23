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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::RouteProfile;

    const GARE_DE_LYON: Coordinates = Coordinates {
        latitude: 48.844_444,
        longitude: 2.373_611,
    };
    const GARE_DE_LILLE: Coordinates = Coordinates {
        latitude: 50.637_9,
        longitude: 3.070_6,
    };

    #[test]
    fn haversine_matches_the_known_paris_lille_distance() {
        let meters = haversine_meters(GARE_DE_LYON, GARE_DE_LILLE);
        // ~204 km as the crow flies; a percent of slack covers the radius model.
        assert!(
            (202_000.0..206_000.0).contains(&meters),
            "unexpected distance: {meters}"
        );
    }

    #[tokio::test]
    async fn a_route_sums_its_legs_and_is_flagged_as_estimated() {
        let route = HaversineRoutingProvider
            .route(RouteRequest::new(
                vec![GARE_DE_LYON, GARE_DE_LILLE, GARE_DE_LYON],
                RouteProfile::Driving,
            ))
            .await
            .expect("the route should be computable");

        assert_eq!(route.legs.len(), 2);
        assert!(route.estimated);
        assert!(route.geometry.is_none());
        let summed: f64 = route.legs.iter().map(|leg| leg.distance_meters).sum();
        assert!((route.distance_meters - summed).abs() < 1e-6);
        assert!(route.duration_seconds > 0.0);
    }

    #[tokio::test]
    async fn a_single_waypoint_is_rejected() {
        let error = HaversineRoutingProvider
            .route(RouteRequest::new(vec![GARE_DE_LYON], RouteProfile::Driving))
            .await
            .expect_err("one waypoint is not a route");

        assert!(matches!(error, crate::error::ApiError::BadRequest(_)));
    }

    #[tokio::test]
    async fn driving_traffic_refuses_more_than_three_waypoints() {
        let error = HaversineRoutingProvider
            .route(RouteRequest::new(
                vec![GARE_DE_LYON; 4],
                RouteProfile::DrivingTraffic,
            ))
            .await
            .expect_err("driving-traffic is capped at three waypoints");

        assert!(matches!(error, crate::error::ApiError::BadRequest(_)));
    }
}
