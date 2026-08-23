use axum::Json;
use axum::extract::{Path, State};
use validator::Validate;

use crate::AppState;
use crate::endpoints::users::AuthenticatedUser;
use crate::error::{ApiError, ApiResult, ErrorResponse};

use super::models::{Location, UpdateLocationRequest};
use super::repository;

/// Update a location
///
/// Fields left out keep their current value. The two coordinates move together:
/// sending only one of them is rejected.
#[utoipa::path(
    patch,
    path = "/{id}",
    operation_id = "updateLocation",
    tag = super::TAG,
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Location identifier")),
    request_body = UpdateLocationRequest,
    responses(
        (status = OK, description = "The updated location", body = Location),
        (status = BAD_REQUEST, description = "Payload failed validation", body = ErrorResponse),
        (status = UNAUTHORIZED, description = "Missing or invalid access token", body = ErrorResponse),
        (status = NOT_FOUND, description = "No location with this id", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    AuthenticatedUser(_user): AuthenticatedUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateLocationRequest>,
) -> ApiResult<Json<Location>> {
    payload.validate()?;

    if payload.latitude.is_some() != payload.longitude.is_some() {
        return Err(ApiError::BadRequest(
            "`latitude` and `longitude` must be updated together".to_string(),
        ));
    }

    if payload.name.is_none() && payload.latitude.is_none() {
        return Err(ApiError::BadRequest("nothing to update".to_string()));
    }

    let record = repository::update_location(
        &state.db,
        id,
        payload.name.as_deref().map(str::trim),
        payload.latitude,
        payload.longitude,
    )
    .await?
    .ok_or(ApiError::NotFound("location not found"))?;

    Ok(Json(record.into()))
}
