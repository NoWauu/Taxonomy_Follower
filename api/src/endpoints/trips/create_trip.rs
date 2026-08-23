use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use validator::Validate;

use crate::AppState;
use crate::endpoints::users::AuthenticatedUser;
use crate::error::{ApiResult, ErrorResponse};

use super::models::{CreateTripRequest, Trip};
use super::repository;
use super::validation;

/// Publish a trip
///
/// The authenticated user becomes the owner of the trip and is the only one
/// allowed to change or cancel it afterwards. Stops are stored in the order
/// they are given, departure and arrival excluded.
#[utoipa::path(
    post,
    path = "",
    operation_id = "createTrip",
    tag = super::TAG,
    security(("bearer_auth" = [])),
    request_body = CreateTripRequest,
    responses(
        (status = CREATED, description = "Trip published", body = Trip),
        (status = BAD_REQUEST, description = "Payload failed validation, or a location is unknown", body = ErrorResponse),
        (status = UNAUTHORIZED, description = "Missing or invalid access token", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
    Json(payload): Json<CreateTripRequest>,
) -> ApiResult<(StatusCode, Json<Trip>)> {
    payload.validate()?;
    validation::ensure_departure_is_future(payload.start_date)?;
    validation::ensure_route_is_valid(
        &state.db,
        payload.start_location_id,
        payload.end_location_id,
        &payload.stop_location_ids,
    )
    .await?;

    let trip_id = repository::insert_trip(
        &state.db,
        user.id,
        payload.start_date,
        payload.start_location_id,
        payload.end_location_id,
        payload.available_seats,
        &payload.stop_location_ids,
    )
    .await?;

    let trip = repository::load_trip(&state.db, trip_id).await?;

    Ok((StatusCode::CREATED, Json(trip)))
}
