//! Route computation between the points of a trip.
//!
//! A trip is a departure, an ordered list of stops and an arrival; turning that
//! into a distance, a duration and a drawable line is what a [`RoutingProvider`]
//! does. The provider is picked once at startup from the environment, exactly
//! like the mail one: with `MAPBOX_ACCESS_TOKEN` set the real Directions API is
//! called, without it routes fall back to straight-line estimates so local
//! development needs no account.

mod haversine;
mod mapbox;

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::config::RoutingConfig;
use crate::error::{ApiError, ApiResult};

pub use haversine::HaversineRoutingProvider;
pub use mapbox::MapboxRoutingProvider;

/// A WGS84 point, in decimal degrees.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Coordinates {
    #[schema(example = 48.844_444, minimum = -90.0, maximum = 90.0)]
    pub latitude: f64,
    #[schema(example = 2.373_611, minimum = -180.0, maximum = 180.0)]
    pub longitude: f64,
}

/// How the route is travelled. Decides both the road network used and, for the
/// fallback provider, the speed distances are turned into durations with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RouteProfile {
    /// Car, using historical and live traffic. Limited to three waypoints.
    DrivingTraffic,
    /// Car, ignoring traffic.
    #[default]
    Driving,
    Walking,
    Cycling,
}

impl RouteProfile {
    /// Slug used in the Mapbox Directions path.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DrivingTraffic => "driving-traffic",
            Self::Driving => "driving",
            Self::Walking => "walking",
            Self::Cycling => "cycling",
        }
    }

    /// Waypoints the profile accepts in a single request. `driving-traffic` is
    /// capped at three by Mapbox, the others at twenty-five.
    pub fn max_waypoints(self) -> usize {
        match self {
            Self::DrivingTraffic => 3,
            _ => 25,
        }
    }

    /// Average speed in metres per second, used to estimate a duration when no
    /// real routing engine is configured.
    fn average_speed_mps(self) -> f64 {
        match self {
            Self::DrivingTraffic | Self::Driving => 70_000.0 / 3_600.0,
            Self::Cycling => 16_000.0 / 3_600.0,
            Self::Walking => 4_500.0 / 3_600.0,
        }
    }
}

impl std::str::FromStr for RouteProfile {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "driving-traffic" | "driving_traffic" => Ok(Self::DrivingTraffic),
            "driving" | "car" => Ok(Self::Driving),
            "walking" | "walk" | "foot" => Ok(Self::Walking),
            "cycling" | "bike" | "bicycle" => Ok(Self::Cycling),
            other => anyhow::bail!(
                "invalid routing profile `{other}`, expected one of: driving-traffic, driving, walking, cycling"
            ),
        }
    }
}

/// Waypoints to visit, in order, plus how to travel between them.
#[derive(Debug, Clone)]
pub struct RouteRequest {
    /// Departure first, arrival last, stops in between. At least two.
    pub waypoints: Vec<Coordinates>,
    pub profile: RouteProfile,
}

impl RouteRequest {
    pub fn new(waypoints: Vec<Coordinates>, profile: RouteProfile) -> Self {
        Self { waypoints, profile }
    }

    /// Rejects requests no provider could answer, so every implementation can
    /// assume a usable waypoint list.
    fn validate(&self) -> ApiResult<()> {
        if self.waypoints.len() < 2 {
            return Err(ApiError::BadRequest(
                "a route needs at least a departure and an arrival".to_string(),
            ));
        }

        let max = self.profile.max_waypoints();
        if self.waypoints.len() > max {
            return Err(ApiError::BadRequest(format!(
                "the `{}` profile accepts at most {max} waypoints, got {}",
                self.profile.as_str(),
                self.waypoints.len()
            )));
        }

        for point in &self.waypoints {
            if !(-90.0..=90.0).contains(&point.latitude)
                || !(-180.0..=180.0).contains(&point.longitude)
            {
                return Err(ApiError::BadRequest(format!(
                    "`{},{}` is not a valid WGS84 coordinate",
                    point.latitude, point.longitude
                )));
            }
        }

        Ok(())
    }
}

/// One hop of the route, between two consecutive waypoints.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RouteLeg {
    #[schema(example = 12_500.4)]
    pub distance_meters: f64,
    #[schema(example = 940.2)]
    pub duration_seconds: f64,
}

/// A computed route: how far, how long, and the line to draw for it.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Route {
    #[schema(example = 42_300.0)]
    pub distance_meters: f64,
    #[schema(example = 2_640.0)]
    pub duration_seconds: f64,
    /// One leg per pair of consecutive waypoints, so `waypoints.len() - 1` of
    /// them, in the order they are travelled.
    pub legs: Vec<RouteLeg>,
    /// The route drawn as a [polyline6]-encoded string, when the provider gives
    /// one. The fallback provider draws nothing.
    ///
    /// [polyline6]: https://github.com/mapbox/polyline
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<String>,
    /// `true` when the numbers come from straight-line distances instead of a
    /// real road network. Never trust them for anything a user pays for.
    pub estimated: bool,
}

#[async_trait]
pub trait RoutingProvider: Send + Sync + 'static {
    async fn route(&self, request: RouteRequest) -> ApiResult<Route>;
}

/// Builds the provider the configuration asks for.
///
/// Falls back to straight-line estimates when no Mapbox token is set, the same
/// way missing SMTP settings downgrade mail delivery to logging.
pub fn from_config(config: &RoutingConfig) -> anyhow::Result<Arc<dyn RoutingProvider>> {
    match &config.mapbox {
        Some(mapbox) => Ok(Arc::new(MapboxRoutingProvider::new(
            mapbox,
            config.timeout,
        )?)),
        None => Ok(Arc::new(HaversineRoutingProvider::new())),
    }
}
