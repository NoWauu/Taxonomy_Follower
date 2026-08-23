use axum::Json;
use axum::extract::{Query, State};
use validator::Validate;

use crate::AppState;
use crate::error::{ApiError, ApiResult, ErrorResponse};

use super::models::{Trip, TripQuery};
use super::repository;

/// Search trips
///
/// Every filter is optional and they compose. `latitude` / `longitude` search
/// around a departure point, keeping trips whose start location lies within
/// `radius_meters` (10 km by default). Results are sorted by departure date.
#[utoipa::path(
    get,
    path = "",
    operation_id = "listTrips",
    tag = super::TAG,
    params(TripQuery),
    responses(
        (status = OK, description = "Matching trips", body = Vec<Trip>),
        (status = BAD_REQUEST, description = "Query parameters failed validation", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    Query(query): Query<TripQuery>,
) -> ApiResult<Json<Vec<Trip>>> {
    query.validate()?;

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

    if let (Some(after), Some(before)) = (query.departing_after, query.departing_before)
        && after > before
    {
        return Err(ApiError::BadRequest(
            "`departing_after` must not be later than `departing_before`".to_string(),
        ));
    }

    let records = repository::list_trips(&state.db, &query).await?;
    let trips = repository::hydrate(&state.db, records).await?;

    Ok(Json(trips))
}
