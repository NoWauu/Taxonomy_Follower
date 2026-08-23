use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::AppState;
use crate::endpoints::users::AuthenticatedUser;
use crate::error::{ApiError, ApiResult, ErrorResponse};

use super::repository;

/// Cancel a trip
///
/// Only the user who published the trip may delete it. Its stops go with it.
#[utoipa::path(
    delete,
    path = "/{id}",
    operation_id = "deleteTrip",
    tag = super::TAG,
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Trip identifier")),
    responses(
        (status = NO_CONTENT, description = "Trip cancelled"),
        (status = UNAUTHORIZED, description = "Missing or invalid access token", body = ErrorResponse),
        (status = FORBIDDEN, description = "The trip belongs to another user", body = ErrorResponse),
        (status = NOT_FOUND, description = "No trip with this id", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
    Path(id): Path<i32>,
) -> ApiResult<StatusCode> {
    let owner = repository::find_trip_owner(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound("trip not found"))?;

    if owner != user.id {
        return Err(ApiError::Forbidden("this trip belongs to another user"));
    }

    if repository::delete_trip(&state.db, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound("trip not found"))
    }
}
