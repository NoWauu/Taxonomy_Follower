//! Every statement touching `trips` and `trip_stops`.
//!
//! Trips are always read together with their departure and arrival locations,
//! whose `GEOGRAPHY(POINT, 4326)` column is projected into plain doubles by the
//! query (`ST_X` / `ST_Y`), never selected raw.
//!
//! The `!` suffixes on the aliases tell the `query_as!` macro that a column is
//! not nullable. They are needed both on the computed coordinates and on the
//! columns coming from the joined `locations` rows: Postgres reports both as
//! nullable, even though the joins are inner ones on `NOT NULL` foreign keys.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

use super::models::{DEFAULT_RADIUS_METERS, Trip, TripQuery, TripRecord, TripStopRecord};

pub async fn find_trip(db: &sqlx::PgPool, id: i32) -> ApiResult<Option<TripRecord>> {
    Ok(sqlx::query_as!(
        TripRecord,
        r#"SELECT t.id,
                  t.created_by,
                  t.start_date,
                  t.available_seats,
                  t.created_at,
                  t.updated_at,
                  s.id   AS "start_location_id!",
                  s.name AS "start_location_name!",
                  ST_Y(s.position::geometry) AS "start_location_latitude!",
                  ST_X(s.position::geometry) AS "start_location_longitude!",
                  s.created_at AS "start_location_created_at!",
                  s.updated_at AS "start_location_updated_at!",
                  e.id   AS "end_location_id!",
                  e.name AS "end_location_name!",
                  ST_Y(e.position::geometry) AS "end_location_latitude!",
                  ST_X(e.position::geometry) AS "end_location_longitude!",
                  e.created_at AS "end_location_created_at!",
                  e.updated_at AS "end_location_updated_at!"
             FROM trips t
             JOIN locations s ON s.id = t.start_location_id
             JOIN locations e ON e.id = t.end_location_id
            WHERE t.id = $1"#,
        id,
    )
    .fetch_optional(db)
    .await?)
}

/// Reads a trip together with its stops, or fails with a 404.
pub async fn load_trip(db: &sqlx::PgPool, id: i32) -> ApiResult<Trip> {
    let record = find_trip(db, id)
        .await?
        .ok_or(ApiError::NotFound("trip not found"))?;

    let stops = find_stops(db, &[record.id]).await?;

    Ok(Trip::from_parts(record, stops))
}

/// Returns the publisher of a trip, or `None` when the trip does not exist.
/// Used by the mutating endpoints to tell 404 apart from 403.
pub async fn find_trip_owner(db: &sqlx::PgPool, id: i32) -> ApiResult<Option<Uuid>> {
    Ok(
        sqlx::query_scalar!("SELECT created_by FROM trips WHERE id = $1", id)
            .fetch_optional(db)
            .await?,
    )
}

/// Lists trips matching `query`.
///
/// The departure proximity filter runs `ST_DWithin` against the *start*
/// location's geography column, so it uses the GiST index and answers in
/// metres. Every filter is a nullable parameter guarded by an `IS NULL` check,
/// which keeps this to a single prepared statement whatever the client sends.
pub async fn list_trips(db: &sqlx::PgPool, query: &TripQuery) -> ApiResult<Vec<TripRecord>> {
    let radius = query.radius_meters.unwrap_or(DEFAULT_RADIUS_METERS);

    Ok(sqlx::query_as!(
        TripRecord,
        r#"SELECT t.id,
                  t.created_by,
                  t.start_date,
                  t.available_seats,
                  t.created_at,
                  t.updated_at,
                  s.id   AS "start_location_id!",
                  s.name AS "start_location_name!",
                  ST_Y(s.position::geometry) AS "start_location_latitude!",
                  ST_X(s.position::geometry) AS "start_location_longitude!",
                  s.created_at AS "start_location_created_at!",
                  s.updated_at AS "start_location_updated_at!",
                  e.id   AS "end_location_id!",
                  e.name AS "end_location_name!",
                  ST_Y(e.position::geometry) AS "end_location_latitude!",
                  ST_X(e.position::geometry) AS "end_location_longitude!",
                  e.created_at AS "end_location_created_at!",
                  e.updated_at AS "end_location_updated_at!"
             FROM trips t
             JOIN locations s ON s.id = t.start_location_id
             JOIN locations e ON e.id = t.end_location_id
            WHERE ($1::int IS NULL OR t.start_location_id = $1::int)
              AND ($2::int IS NULL OR t.end_location_id = $2::int)
              AND ($3::timestamptz IS NULL OR t.start_date >= $3::timestamptz)
              AND ($4::timestamptz IS NULL OR t.start_date <= $4::timestamptz)
              AND ($5::int IS NULL OR t.available_seats >= $5::int)
              AND ($6::uuid IS NULL OR t.created_by = $6::uuid)
              AND (
                   $7::double precision IS NULL
                   OR ST_DWithin(
                          s.position,
                          ST_SetSRID(ST_MakePoint($8::double precision, $7::double precision), 4326)::geography,
                          $9::double precision
                      )
                  )
            ORDER BY t.start_date ASC, t.id ASC
            LIMIT $10 OFFSET $11"#,
        query.start_location_id,
        query.end_location_id,
        query.departing_after,
        query.departing_before,
        query.min_available_seats,
        query.created_by,
        query.latitude,
        query.longitude,
        radius,
        query.limit,
        query.offset,
    )
    .fetch_all(db)
    .await?)
}

/// Loads the stops of several trips in one round trip, so listing trips stays
/// two queries instead of one per trip.
pub async fn find_stops(db: &sqlx::PgPool, trip_ids: &[i32]) -> ApiResult<Vec<TripStopRecord>> {
    if trip_ids.is_empty() {
        return Ok(Vec::new());
    }

    Ok(sqlx::query_as!(
        TripStopRecord,
        r#"SELECT ts.id,
                  ts.trip_id,
                  ts.stop_order,
                  l.id   AS "location_id!",
                  l.name AS "location_name!",
                  ST_Y(l.position::geometry) AS "location_latitude!",
                  ST_X(l.position::geometry) AS "location_longitude!",
                  l.created_at AS "location_created_at!",
                  l.updated_at AS "location_updated_at!"
             FROM trip_stops ts
             JOIN locations l ON l.id = ts.stop_location_id
            WHERE ts.trip_id = ANY($1::int[])
            ORDER BY ts.trip_id ASC, ts.stop_order ASC"#,
        trip_ids,
    )
    .fetch_all(db)
    .await?)
}

/// Attaches the stops to their trips, preserving the order of `records`.
pub async fn hydrate(db: &sqlx::PgPool, records: Vec<TripRecord>) -> ApiResult<Vec<Trip>> {
    let ids: Vec<i32> = records.iter().map(|record| record.id).collect();
    let stops = find_stops(db, &ids).await?;

    let mut by_trip: HashMap<i32, Vec<TripStopRecord>> = HashMap::new();
    for stop in stops {
        by_trip.entry(stop.trip_id).or_default().push(stop);
    }

    Ok(records
        .into_iter()
        .map(|record| {
            let stops = by_trip.remove(&record.id).unwrap_or_default();
            Trip::from_parts(record, stops)
        })
        .collect())
}

/// Inserts a trip and its ordered stops atomically, returning the new id.
pub async fn insert_trip(
    db: &sqlx::PgPool,
    created_by: Uuid,
    start_date: DateTime<Utc>,
    start_location_id: i32,
    end_location_id: i32,
    available_seats: i32,
    stop_location_ids: &[i32],
) -> ApiResult<i32> {
    let mut tx = db.begin().await?;

    let trip_id = sqlx::query_scalar!(
        "INSERT INTO trips (created_by, start_date, start_location_id, end_location_id, available_seats)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id",
        created_by,
        start_date,
        start_location_id,
        end_location_id,
        available_seats,
    )
    .fetch_one(&mut *tx)
    .await?;

    insert_stops(&mut tx, trip_id, stop_location_ids).await?;

    tx.commit().await?;

    Ok(trip_id)
}

/// Applies a partial update and, when `stop_location_ids` is given, replaces
/// the stop list wholesale. Returns `false` if the trip no longer exists.
pub async fn update_trip(
    db: &sqlx::PgPool,
    id: i32,
    start_date: Option<DateTime<Utc>>,
    start_location_id: Option<i32>,
    end_location_id: Option<i32>,
    available_seats: Option<i32>,
    stop_location_ids: Option<&[i32]>,
) -> ApiResult<bool> {
    let mut tx = db.begin().await?;

    let updated = sqlx::query_scalar!(
        "UPDATE trips
            SET start_date        = COALESCE($2::timestamptz, start_date),
                start_location_id = COALESCE($3::int, start_location_id),
                end_location_id   = COALESCE($4::int, end_location_id),
                available_seats   = COALESCE($5::int, available_seats)
          WHERE id = $1
          RETURNING id",
        id,
        start_date,
        start_location_id,
        end_location_id,
        available_seats,
    )
    .fetch_optional(&mut *tx)
    .await?;

    if updated.is_none() {
        tx.rollback().await?;
        return Ok(false);
    }

    // A stop list is replaced as a whole: reordering or removing a single stop
    // otherwise collides with the (trip_id, stop_order) unique constraint.
    if let Some(stops) = stop_location_ids {
        sqlx::query!("DELETE FROM trip_stops WHERE trip_id = $1", id)
            .execute(&mut *tx)
            .await?;

        insert_stops(&mut tx, id, stops).await?;
    }

    tx.commit().await?;

    Ok(true)
}

/// Deletes a trip. `trip_stops` cascades.
pub async fn delete_trip(db: &sqlx::PgPool, id: i32) -> ApiResult<bool> {
    let result = sqlx::query!("DELETE FROM trips WHERE id = $1", id)
        .execute(db)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Writes the whole ordered stop list in one statement.
///
/// `unnest(... ) WITH ORDINALITY` derives `stop_order` from the position in the
/// array, so the order the client sent is the order that gets stored.
async fn insert_stops(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    trip_id: i32,
    stop_location_ids: &[i32],
) -> ApiResult<()> {
    if stop_location_ids.is_empty() {
        return Ok(());
    }

    sqlx::query!(
        "INSERT INTO trip_stops (trip_id, stop_location_id, stop_order)
         SELECT $1, stop.location_id, (stop.ordinality - 1)::int
           FROM unnest($2::int[]) WITH ORDINALITY AS stop(location_id, ordinality)",
        trip_id,
        stop_location_ids,
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}
