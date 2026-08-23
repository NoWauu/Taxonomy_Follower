//! Route checks shared by trip creation and trip updates.
//!
//! The database already guards the invariants it can express (`available_seats
//! >= 0`, unique `stop_order`), but a constraint violation would surface as a
//! 500. These checks run first so the client gets a 400 naming what is wrong.

use chrono::{DateTime, Utc};
use std::collections::HashSet;

use crate::endpoints::locations::repository as locations;
use crate::error::{ApiError, ApiResult};

pub fn ensure_departure_is_future(start_date: DateTime<Utc>) -> ApiResult<()> {
    if start_date <= Utc::now() {
        return Err(ApiError::BadRequest(
            "`start_date` must be in the future".to_string(),
        ));
    }

    Ok(())
}

/// Checks the shape of the route, then that every location it names exists.
///
/// Departure and arrival are allowed to be the same location, which is how a
/// round trip is expressed: it leaves from and returns to one place, and the
/// stops in between describe the actual route.
///
/// The existence check is one query for all ids: without it the insert would
/// fail on a foreign key and turn into an opaque 500.
pub async fn ensure_route_is_valid(
    db: &sqlx::PgPool,
    start_location_id: i32,
    end_location_id: i32,
    stop_location_ids: &[i32],
) -> ApiResult<()> {
    let mut seen = HashSet::with_capacity(stop_location_ids.len());
    for id in stop_location_ids {
        if !seen.insert(*id) {
            return Err(ApiError::BadRequest(format!(
                "location {id} appears twice in `stop_location_ids`"
            )));
        }

        // A round trip is start == end, so this rejects a stop that repeats
        // either endpoint, not the endpoints matching each other.
        if *id == start_location_id || *id == end_location_id {
            return Err(ApiError::BadRequest(format!(
                "location {id} is already the departure or the arrival and cannot be a stop"
            )));
        }
    }

    let mut referenced = vec![start_location_id, end_location_id];
    referenced.extend_from_slice(stop_location_ids);

    let missing = locations::find_missing_ids(db, &referenced).await?;
    if !missing.is_empty() {
        let list = missing
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join(", ");

        return Err(ApiError::BadRequest(format!("unknown location ids: {list}")));
    }

    Ok(())
}
