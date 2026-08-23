use axum::Json;
use axum::extract::{Query, State};
use validator::Validate;

use crate::AppState;
use crate::error::{ApiError, ApiResult, ErrorResponse};

use super::models::{Location, LocationQuery};
use super::repository;

/// List locations
///
/// Supports a name substring filter and a proximity search. Passing
/// `latitude` and `longitude` restricts the result to the disc of
/// `radius_meters` (10 km by default) around that point and sorts it by growing
/// distance, with the distance reported back in `distance_meters`.
#[utoipa::path(
    get,
    path = "",
    operation_id = "listLocations",
    tag = super::TAG,
    params(LocationQuery),
    responses(
        (status = OK, description = "Matching locations", body = Vec<Location>),
        (status = BAD_REQUEST, description = "Query parameters failed validation", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    Query(query): Query<LocationQuery>,
) -> ApiResult<Json<Vec<Location>>> {
    query.validate()?;

    // Half a point is not a point: refuse rather than silently ignoring one.
    if query.latitude.is_some() != query.longitude.is_some() {
        return Err(ApiError::BadRequest(
            "`latitude` and `longitude` must be provided together".to_string(),
        ));
    }

    if query.radius_meters.is_some() && query.latitude.is_none() {
        return Err(ApiError::BadRequest(
            "`radius_meters` requires `latitude` and `longitude`".to_string(),
        ));
    }

    let records = repository::list_locations(&state.db, &query).await?;

    Ok(Json(records.into_iter().map(Location::from).collect()))
}
