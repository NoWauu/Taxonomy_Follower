use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use crate::endpoints::locations::Location;

/// A `trips` row joined with its departure and arrival locations.
///
/// The two `GEOGRAPHY` columns are projected into doubles by the query; see
/// `endpoints::locations::repository` for why.
#[derive(Debug, Clone)]
pub struct TripRecord {
    pub id: i32,
    pub created_by: Uuid,
    pub start_date: DateTime<Utc>,
    pub available_seats: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    pub start_location_id: i32,
    pub start_location_name: String,
    pub start_location_latitude: f64,
    pub start_location_longitude: f64,
    pub start_location_created_at: DateTime<Utc>,
    pub start_location_updated_at: DateTime<Utc>,

    pub end_location_id: i32,
    pub end_location_name: String,
    pub end_location_latitude: f64,
    pub end_location_longitude: f64,
    pub end_location_created_at: DateTime<Utc>,
    pub end_location_updated_at: DateTime<Utc>,
}

/// A `trip_stops` row joined with the location it points at.
#[derive(Debug, Clone)]
pub struct TripStopRecord {
    pub id: i32,
    pub trip_id: i32,
    pub stop_order: i32,
    pub location_id: i32,
    pub location_name: String,
    pub location_latitude: f64,
    pub location_longitude: f64,
    pub location_created_at: DateTime<Utc>,
    pub location_updated_at: DateTime<Utc>,
}

/// An intermediate stop, ordered along the route.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TripStop {
    pub id: i32,
    /// Rank of the stop along the route, 0-based, departure and arrival excluded.
    pub stop_order: i32,
    pub location: Location,
}

impl From<TripStopRecord> for TripStop {
    fn from(record: TripStopRecord) -> Self {
        Self {
            id: record.id,
            stop_order: record.stop_order,
            location: Location {
                id: record.location_id,
                name: record.location_name,
                latitude: record.location_latitude,
                longitude: record.location_longitude,
                distance_meters: None,
                created_at: record.location_created_at,
                updated_at: record.location_updated_at,
            },
        }
    }
}

/// A ride offered by a user: when it leaves, where from, where to, which stops
/// it makes on the way, and how many seats are still open.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Trip {
    pub id: i32,
    /// The user who published the trip. Only they may edit or cancel it.
    pub created_by: Uuid,
    /// Departure instant, in UTC.
    pub start_date: DateTime<Utc>,
    pub start_location: Location,
    pub end_location: Location,
    /// Intermediate stops, sorted by `stop_order`.
    pub stops: Vec<TripStop>,
    /// Seats still bookable.
    #[schema(example = 3, minimum = 0)]
    pub available_seats: i32,
    /// When the trip was published.
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Trip {
    pub fn from_parts(record: TripRecord, stops: Vec<TripStopRecord>) -> Self {
        Self {
            id: record.id,
            created_by: record.created_by,
            start_date: record.start_date,
            start_location: Location {
                id: record.start_location_id,
                name: record.start_location_name,
                latitude: record.start_location_latitude,
                longitude: record.start_location_longitude,
                distance_meters: None,
                created_at: record.start_location_created_at,
                updated_at: record.start_location_updated_at,
            },
            end_location: Location {
                id: record.end_location_id,
                name: record.end_location_name,
                latitude: record.end_location_latitude,
                longitude: record.end_location_longitude,
                distance_meters: None,
                created_at: record.end_location_created_at,
                updated_at: record.end_location_updated_at,
            },
            stops: stops.into_iter().map(TripStop::from).collect(),
            available_seats: record.available_seats,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateTripRequest {
    /// Departure instant, in UTC. Must be in the future.
    #[schema(example = "2026-09-01T07:30:00Z")]
    pub start_date: DateTime<Utc>,

    #[validate(range(min = 1, message = "must be a valid location id"))]
    pub start_location_id: i32,

    #[validate(range(min = 1, message = "must be a valid location id"))]
    pub end_location_id: i32,

    #[validate(range(min = 0, max = 64, message = "must be between 0 and 64"))]
    #[schema(example = 3, minimum = 0, maximum = 64)]
    pub available_seats: i32,

    /// Locations the trip stops at on the way, in the order they are visited.
    /// Departure and arrival are implicit and must not be repeated here.
    #[serde(default)]
    #[validate(length(max = 32, message = "at most 32 stops"))]
    #[schema(example = json!([12, 15]), max_items = 32)]
    pub stop_location_ids: Vec<i32>,
}

/// Partial update of a trip. Omitted fields keep their value; passing
/// `stop_location_ids` replaces the whole ordered list.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateTripRequest {
    #[schema(example = "2026-09-01T07:30:00Z")]
    pub start_date: Option<DateTime<Utc>>,

    #[validate(range(min = 1, message = "must be a valid location id"))]
    pub start_location_id: Option<i32>,

    #[validate(range(min = 1, message = "must be a valid location id"))]
    pub end_location_id: Option<i32>,

    #[validate(range(min = 0, max = 64, message = "must be between 0 and 64"))]
    #[schema(example = 2, minimum = 0, maximum = 64)]
    pub available_seats: Option<i32>,

    #[validate(length(max = 32, message = "at most 32 stops"))]
    #[schema(max_items = 32)]
    pub stop_location_ids: Option<Vec<i32>>,
}

fn default_limit() -> i64 {
    50
}

/// Filters accepted by `GET /trips`.
///
/// `latitude` / `longitude` / `radius_meters` search around a *departure*
/// point: they keep the trips whose start location falls inside the disc.
#[derive(Debug, Deserialize, Validate, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TripQuery {
    /// Only trips leaving from this exact location.
    pub start_location_id: Option<i32>,

    /// Only trips arriving at this exact location.
    pub end_location_id: Option<i32>,

    /// Only trips whose start location is within `radius_meters` of this point.
    #[validate(range(min = -90.0, max = 90.0, message = "must be between -90 and 90"))]
    #[param(example = 48.844_444, minimum = -90.0, maximum = 90.0)]
    pub latitude: Option<f64>,

    #[validate(range(min = -180.0, max = 180.0, message = "must be between -180 and 180"))]
    #[param(example = 2.373_611, minimum = -180.0, maximum = 180.0)]
    pub longitude: Option<f64>,

    /// Radius of the departure search in metres, 10 km by default.
    #[validate(range(min = 1.0, max = 500_000.0, message = "must be between 1 and 500000"))]
    #[param(example = 10_000.0, minimum = 1.0, maximum = 500_000.0)]
    pub radius_meters: Option<f64>,

    /// Only trips leaving at or after this instant.
    #[param(example = "2026-09-01T00:00:00Z")]
    pub departing_after: Option<DateTime<Utc>>,

    /// Only trips leaving at or before this instant.
    #[param(example = "2026-09-30T23:59:59Z")]
    pub departing_before: Option<DateTime<Utc>>,

    /// Only trips with at least this many seats left.
    #[validate(range(min = 0, max = 64, message = "must be between 0 and 64"))]
    #[param(example = 1, minimum = 0, maximum = 64)]
    pub min_available_seats: Option<i32>,

    /// Only trips published by this user.
    pub created_by: Option<Uuid>,

    #[validate(range(min = 1, max = 200, message = "must be between 1 and 200"))]
    #[serde(default = "default_limit")]
    #[param(default = 50, minimum = 1, maximum = 200)]
    pub limit: i64,

    #[validate(range(min = 0, message = "must not be negative"))]
    #[serde(default)]
    #[param(default = 0, minimum = 0)]
    pub offset: i64,
}

pub const DEFAULT_RADIUS_METERS: f64 = 10_000.0;
