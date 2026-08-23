use axum::Json;
use axum::extract::{Path, State};
use validator::Validate;

use crate::AppState;
use crate::endpoints::users::AuthenticatedUser;
use crate::error::{ApiError, ApiResult, ErrorResponse};

use super::models::{Trip, UpdateTripRequest};
use super::repository;
use super::validation;

/// Update a trip
///
/// Only the user who published the trip may change it. Omitted fields keep
/// their value; passing `stop_location_ids` replaces the whole ordered list,
/// and passing an empty array clears the stops.
#[utoipa::path(
    patch,
    path = "/{id}",
    operation_id = "updateTrip",
    tag = super::TAG,
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Trip identifier")),
    request_body = UpdateTripRequest,
    responses(
        (status = OK, description = "The updated trip", body = Trip),
        (status = BAD_REQUEST, description = "Payload failed validation, or a location is unknown", body = ErrorResponse),
        (status = UNAUTHORIZED, description = "Missing or invalid access token", body = ErrorResponse),
        (status = FORBIDDEN, description = "The trip belongs to another user", body = ErrorResponse),
        (status = NOT_FOUND, description = "No trip with this id", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateTripRequest>,
) -> ApiResult<Json<Trip>> {
    payload.validate()?;

    if payload.start_date.is_none()
        && payload.start_location_id.is_none()
        && payload.end_location_id.is_none()
        && payload.available_seats.is_none()
        && payload.stop_location_ids.is_none()
    {
        return Err(ApiError::BadRequest("nothing to update".to_string()));
    }

    let owner = repository::find_trip_owner(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound("trip not found"))?;

    if owner != user.id {
        return Err(ApiError::Forbidden("this trip belongs to another user"));
    }

    if let Some(start_date) = payload.start_date {
        validation::ensure_departure_is_future(start_date)?;
    }

    // The route is validated as it will be *after* the update, so a patch that
    // only moves the arrival is still checked against the existing stops.
    let current = repository::load_trip(&state.db, id).await?;

    let start_location_id = payload
        .start_location_id
        .unwrap_or(current.start_location.id);
    let end_location_id = payload.end_location_id.unwrap_or(current.end_location.id);
    let stop_location_ids = payload.stop_location_ids.clone().unwrap_or_else(|| {
        current
            .stops
            .iter()
            .map(|stop| stop.location.id)
            .collect::<Vec<_>>()
    });

    validation::ensure_route_is_valid(
        &state.db,
        start_location_id,
        end_location_id,
        &stop_location_ids,
    )
    .await?;

    let updated = repository::update_trip(
        &state.db,
        id,
        payload.start_date,
        payload.start_location_id,
        payload.end_location_id,
        payload.available_seats,
        payload.stop_location_ids.as_deref(),
    )
    .await?;

    if !updated {
        return Err(ApiError::NotFound("trip not found"));
    }

    Ok(Json(repository::load_trip(&state.db, id).await?))
}
