//! Every statement touching the `locations` table.
//!
//! `position` is a `GEOGRAPHY(POINT, 4326)` and sqlx has no PostGIS decoder, so
//! it is never selected raw. Reads project it with `ST_Y(position::geometry)`
//! for the latitude and `ST_X(position::geometry)` for the longitude; writes
//! rebuild it with `ST_SetSRID(ST_MakePoint(<lon>, <lat>), 4326)::geography`.
//! Mind the argument order of `ST_MakePoint`: it takes (x, y) = (lon, lat).
//!
//! The `!` suffixes on the aliases tell the `query_as!` macro that a computed
//! column is not nullable; without them every projected coordinate would come
//! back as an `Option<f64>`, since Postgres reports expressions as nullable.

use crate::error::{ApiError, ApiResult};

use super::models::{DEFAULT_RADIUS_METERS, LocationQuery, LocationRecord};

const FOREIGN_KEY_VIOLATION_SQLSTATE: &str = "23503";

pub async fn insert_location(
    db: &sqlx::PgPool,
    name: &str,
    latitude: f64,
    longitude: f64,
) -> ApiResult<LocationRecord> {
    Ok(sqlx::query_as!(
        LocationRecord,
        r#"INSERT INTO locations (name, position)
           VALUES ($1, ST_SetSRID(ST_MakePoint($2::double precision, $3::double precision), 4326)::geography)
           RETURNING id,
                     name,
                     ST_Y(position::geometry) AS "latitude!",
                     ST_X(position::geometry) AS "longitude!",
                     NULL::double precision AS "distance_meters",
                     created_at,
                     updated_at"#,
        name,
        longitude,
        latitude,
    )
    .fetch_one(db)
    .await?)
}

pub async fn find_location(db: &sqlx::PgPool, id: i32) -> ApiResult<Option<LocationRecord>> {
    Ok(sqlx::query_as!(
        LocationRecord,
        r#"SELECT id,
                  name,
                  ST_Y(position::geometry) AS "latitude!",
                  ST_X(position::geometry) AS "longitude!",
                  NULL::double precision AS "distance_meters",
                  created_at,
                  updated_at
             FROM locations
            WHERE id = $1"#,
        id,
    )
    .fetch_optional(db)
    .await?)
}

/// Lists locations, optionally filtered by name and by proximity to a point.
///
/// The proximity branch is expressed with `ST_DWithin` on the geography column
/// (metres, spheroid) rather than a bounding box, so the GiST index on
/// `position` is used and no results are lost near the antimeridian or the
/// poles. Both branches live in one statement guarded by `IS NULL` checks,
/// which keeps a single prepared statement in the cache.
///
/// The name filter uses `strpos` rather than `ILIKE` so that `%` and `_` typed
/// by a user stay literal characters instead of becoming wildcards.
pub async fn list_locations(
    db: &sqlx::PgPool,
    query: &LocationQuery,
) -> ApiResult<Vec<LocationRecord>> {
    let radius = query.radius_meters.unwrap_or(DEFAULT_RADIUS_METERS);

    Ok(sqlx::query_as!(
        LocationRecord,
        r#"SELECT id,
                  name,
                  ST_Y(position::geometry) AS "latitude!",
                  ST_X(position::geometry) AS "longitude!",
                  CASE
                      WHEN $2::double precision IS NULL THEN NULL::double precision
                      ELSE ST_Distance(
                               position,
                               ST_SetSRID(ST_MakePoint($3::double precision, $2::double precision), 4326)::geography
                           )
                  END AS "distance_meters",
                  created_at,
                  updated_at
             FROM locations
            WHERE ($1::text IS NULL OR strpos(lower(name), lower($1::text)) > 0)
              AND (
                   $2::double precision IS NULL
                   OR ST_DWithin(
                          position,
                          ST_SetSRID(ST_MakePoint($3::double precision, $2::double precision), 4326)::geography,
                          $4::double precision
                      )
                  )
            ORDER BY "distance_meters" ASC NULLS LAST, name ASC, id ASC
            LIMIT $5 OFFSET $6"#,
        query.q.as_deref(),
        query.latitude,
        query.longitude,
        radius,
        query.limit,
        query.offset,
    )
    .fetch_all(db)
    .await?)
}

/// Applies a partial update. `latitude` and `longitude` are expected to be both
/// present or both absent; the caller enforces that before getting here.
pub async fn update_location(
    db: &sqlx::PgPool,
    id: i32,
    name: Option<&str>,
    latitude: Option<f64>,
    longitude: Option<f64>,
) -> ApiResult<Option<LocationRecord>> {
    Ok(sqlx::query_as!(
        LocationRecord,
        r#"UPDATE locations
              SET name = COALESCE($2::varchar, name),
                  position = CASE
                      WHEN $3::double precision IS NULL THEN position
                      ELSE ST_SetSRID(ST_MakePoint($4::double precision, $3::double precision), 4326)::geography
                  END
            WHERE id = $1
            RETURNING id,
                      name,
                      ST_Y(position::geometry) AS "latitude!",
                      ST_X(position::geometry) AS "longitude!",
                      NULL::double precision AS "distance_meters",
                      created_at,
                      updated_at"#,
        id,
        name,
        latitude,
        longitude,
    )
    .fetch_optional(db)
    .await?)
}

/// Deletes a location. Trips and stops reference it with `ON DELETE RESTRICT`,
/// so a location still in use surfaces as a conflict rather than a 500.
pub async fn delete_location(db: &sqlx::PgPool, id: i32) -> ApiResult<bool> {
    let result = sqlx::query!("DELETE FROM locations WHERE id = $1", id)
        .execute(db)
        .await
        .map_err(|error| match &error {
            sqlx::Error::Database(db_error)
                if db_error.code().as_deref() == Some(FOREIGN_KEY_VIOLATION_SQLSTATE) =>
            {
                ApiError::Conflict("this location is still referenced by a trip")
            }
            _ => error.into(),
        })?;

    Ok(result.rows_affected() > 0)
}

/// Returns the ids among `ids` that do not exist. Used to turn an unknown
/// location into a 400 instead of a foreign key error at insert time.
pub async fn find_missing_ids(db: &sqlx::PgPool, ids: &[i32]) -> ApiResult<Vec<i32>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    Ok(sqlx::query_scalar!(
        r#"SELECT candidate AS "candidate!"
             FROM unnest($1::int[]) AS candidate
            WHERE NOT EXISTS (SELECT 1 FROM locations WHERE locations.id = candidate)"#,
        ids,
    )
    .fetch_all(db)
    .await?)
}
