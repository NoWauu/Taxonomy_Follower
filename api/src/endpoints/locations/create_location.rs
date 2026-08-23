use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use validator::Validate;

use crate::AppState;
use crate::endpoints::users::AuthenticatedUser;
use crate::error::{ApiResult, ErrorResponse};

use super::models::{CreateLocationRequest, Location};
use super::repository;

/// Create a location
///
/// Locations are shared reference data: a trip points at them for its
/// departure, its arrival and each of its stops. Coordinates are stored as a
/// WGS84 `GEOGRAPHY(POINT, 4326)`.
#[utoipa::path(
    post,
    path = "",
    operation_id = "createLocation",
    tag = super::TAG,
    security(("bearer_auth" = [])),
    request_body = CreateLocationRequest,
    responses(
        (status = CREATED, description = "Location created", body = Location),
        (status = BAD_REQUEST, description = "Payload failed validation", body = ErrorResponse),
        (status = UNAUTHORIZED, description = "Missing or invalid access token", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    AuthenticatedUser(_user): AuthenticatedUser,
    Json(payload): Json<CreateLocationRequest>,
) -> ApiResult<(StatusCode, Json<Location>)> {
    payload.validate()?;

    let record = repository::insert_location(
        &state.db,
        payload.name.trim(),
        payload.latitude,
        payload.longitude,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(record.into())))
}
