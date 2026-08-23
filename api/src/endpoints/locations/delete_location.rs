use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::AppState;
use crate::endpoints::users::AuthenticatedUser;
use crate::error::{ApiError, ApiResult, ErrorResponse};

use super::repository;

/// Delete a location
///
/// Refused with a conflict while a trip still departs from, stops at or ends at
/// this location.
#[utoipa::path(
    delete,
    path = "/{id}",
    operation_id = "deleteLocation",
    tag = super::TAG,
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Location identifier")),
    responses(
        (status = NO_CONTENT, description = "Location deleted"),
        (status = UNAUTHORIZED, description = "Missing or invalid access token", body = ErrorResponse),
        (status = NOT_FOUND, description = "No location with this id", body = ErrorResponse),
        (status = CONFLICT, description = "Location is still referenced by a trip", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    AuthenticatedUser(_user): AuthenticatedUser,
    Path(id): Path<i32>,
) -> ApiResult<StatusCode> {
    if repository::delete_location(&state.db, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound("location not found"))
    }
}
