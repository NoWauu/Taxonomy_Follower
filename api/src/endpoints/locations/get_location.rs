use axum::Json;
use axum::extract::{Path, State};

use crate::AppState;
use crate::error::{ApiError, ApiResult, ErrorResponse};

use super::models::Location;
use super::repository;

/// Fetch a single location
#[utoipa::path(
    get,
    path = "/{id}",
    operation_id = "getLocation",
    tag = super::TAG,
    params(("id" = i32, Path, description = "Location identifier")),
    responses(
        (status = OK, description = "The location", body = Location),
        (status = NOT_FOUND, description = "No location with this id", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> ApiResult<Json<Location>> {
    let record = repository::find_location(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound("location not found"))?;

    Ok(Json(record.into()))
}
