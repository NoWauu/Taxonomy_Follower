use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

/// A row of `locations` with its `GEOGRAPHY` column already decomposed.
///
/// The `position` column is never selected as-is: sqlx has no PostGIS type, so
/// every query projects it through `ST_X` / `ST_Y` into plain doubles.
/// `distance_meters` is only populated by the proximity search.
#[derive(Debug, Clone)]
pub struct LocationRecord {
    pub id: i32,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub distance_meters: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A place a trip can start from, stop at or end in.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Location {
    pub id: i32,
    #[schema(example = "Gare de Lyon")]
    pub name: String,
    /// WGS84 latitude in decimal degrees.
    #[schema(example = 48.844_444)]
    pub latitude: f64,
    /// WGS84 longitude in decimal degrees.
    #[schema(example = 2.373_611)]
    pub longitude: f64,
    /// Distance to the searched point, in metres. Only set when the request
    /// carried `latitude` / `longitude` query parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_meters: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<LocationRecord> for Location {
    fn from(record: LocationRecord) -> Self {
        Self {
            id: record.id,
            name: record.name,
            latitude: record.latitude,
            longitude: record.longitude,
            distance_meters: record.distance_meters,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateLocationRequest {
    #[validate(length(min = 1, max = 255, message = "must be between 1 and 255 characters"))]
    #[schema(example = "Gare de Lyon", min_length = 1, max_length = 255)]
    pub name: String,

    #[validate(range(min = -90.0, max = 90.0, message = "must be between -90 and 90"))]
    #[schema(example = 48.844_444, minimum = -90.0, maximum = 90.0)]
    pub latitude: f64,

    #[validate(range(min = -180.0, max = 180.0, message = "must be between -180 and 180"))]
    #[schema(example = 2.373_611, minimum = -180.0, maximum = 180.0)]
    pub longitude: f64,
}

/// Partial update. Coordinates move together: sending only one of the two is
/// rejected, since half a point is not a point.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateLocationRequest {
    #[validate(length(min = 1, max = 255, message = "must be between 1 and 255 characters"))]
    #[schema(example = "Gare de Lyon", min_length = 1, max_length = 255)]
    pub name: Option<String>,

    #[validate(range(min = -90.0, max = 90.0, message = "must be between -90 and 90"))]
    #[schema(example = 48.844_444, minimum = -90.0, maximum = 90.0)]
    pub latitude: Option<f64>,

    #[validate(range(min = -180.0, max = 180.0, message = "must be between -180 and 180"))]
    #[schema(example = 2.373_611, minimum = -180.0, maximum = 180.0)]
    pub longitude: Option<f64>,
}

fn default_limit() -> i64 {
    50
}

/// Filters accepted by `GET /locations`.
///
/// `latitude`, `longitude` and `radius_meters` form the proximity search: the
/// two coordinates must be given together, and results are then restricted to
/// the disc of `radius_meters` around them and sorted by growing distance.
#[derive(Debug, Deserialize, Validate, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct LocationQuery {
    /// Case-insensitive substring match on the name.
    #[validate(length(min = 1, max = 255))]
    #[param(example = "gare")]
    pub q: Option<String>,

    #[validate(range(min = -90.0, max = 90.0, message = "must be between -90 and 90"))]
    #[param(example = 48.844_444, minimum = -90.0, maximum = 90.0)]
    pub latitude: Option<f64>,

    #[validate(range(min = -180.0, max = 180.0, message = "must be between -180 and 180"))]
    #[param(example = 2.373_611, minimum = -180.0, maximum = 180.0)]
    pub longitude: Option<f64>,

    /// Search radius in metres, defaults to 10 km when a point is given.
    #[validate(range(min = 1.0, max = 500_000.0, message = "must be between 1 and 500000"))]
    #[param(example = 10_000.0, minimum = 1.0, maximum = 500_000.0)]
    pub radius_meters: Option<f64>,

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
