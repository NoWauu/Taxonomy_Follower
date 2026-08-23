use axum::Json;
use axum::extract::{Path, State};

use crate::AppState;
use crate::error::{ApiResult, ErrorResponse};

use super::models::Trip;
use super::repository;

/// Fetch a single trip
///
/// Returns the trip with its departure, its arrival and its ordered stops.
#[utoipa::path(
    get,
    path = "/{id}",
    operation_id = "getTrip",
    tag = super::TAG,
    params(("id" = i32, Path, description = "Trip identifier")),
    responses(
        (status = OK, description = "The trip", body = Trip),
        (status = NOT_FOUND, description = "No trip with this id", body = ErrorResponse),
    ),
)]
pub async fn handler(State(state): State<AppState>, Path(id): Path<i32>) -> ApiResult<Json<Trip>> {
    Ok(Json(repository::load_trip(&state.db, id).await?))
}
