use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

use super::super::models::UserRecord;

const UNIQUE_VIOLATION_SQLSTATE: &str = "23505";

pub async fn insert_user(
    db: &sqlx::PgPool,
    email: &str,
    password_hash: &str,
    display_name: Option<&str>,
) -> ApiResult<UserRecord> {
    sqlx::query_as::<_, UserRecord>(
        "INSERT INTO users (email, password_hash, display_name)
         VALUES ($1, $2, $3)
         RETURNING id, email, password_hash, display_name, email_verified, created_at, updated_at",
    )
    .bind(email)
    .bind(password_hash)
    .bind(display_name)
    .fetch_one(db)
    .await
    .map_err(|error| match &error {
        sqlx::Error::Database(db_error)
            if db_error.code().as_deref() == Some(UNIQUE_VIOLATION_SQLSTATE) =>
        {
            ApiError::Conflict("an account with this email already exists")
        }
        _ => error.into(),
    })
}

pub async fn find_user_by_email(db: &sqlx::PgPool, email: &str) -> ApiResult<Option<UserRecord>> {
    Ok(sqlx::query_as::<_, UserRecord>(
        "SELECT id, email, password_hash, display_name, email_verified, created_at, updated_at
           FROM users
          WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(db)
    .await?)
}

pub async fn find_user_by_id(db: &sqlx::PgPool, id: Uuid) -> ApiResult<Option<UserRecord>> {
    Ok(sqlx::query_as::<_, UserRecord>(
        "SELECT id, email, password_hash, display_name, email_verified, created_at, updated_at
           FROM users
          WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await?)
}

pub async fn update_password(
    db: &sqlx::PgPool,
    user_id: Uuid,
    password_hash: &str,
) -> ApiResult<()> {
    sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
        .bind(user_id)
        .bind(password_hash)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn insert_refresh_token(
    db: &sqlx::PgPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> ApiResult<()> {
    sqlx::query("INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn consume_refresh_token(db: &sqlx::PgPool, token_hash: &str) -> ApiResult<Option<Uuid>> {
    let user_id: Option<(Uuid,)> = sqlx::query_as(
        "UPDATE refresh_tokens
            SET revoked_at = now()
          WHERE token_hash = $1
            AND revoked_at IS NULL
            AND expires_at > now()
      RETURNING user_id",
    )
    .bind(token_hash)
    .fetch_optional(db)
    .await?;

    Ok(user_id.map(|(id,)| id))
}

pub async fn revoke_refresh_token(db: &sqlx::PgPool, token_hash: &str) -> ApiResult<bool> {
    let result = sqlx::query(
        "UPDATE refresh_tokens
            SET revoked_at = now()
          WHERE token_hash = $1
            AND revoked_at IS NULL",
    )
    .bind(token_hash)
    .execute(db)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn revoke_all_refresh_tokens(db: &sqlx::PgPool, user_id: Uuid) -> ApiResult<u64> {
    let result = sqlx::query(
        "UPDATE refresh_tokens
            SET revoked_at = now()
          WHERE user_id = $1
            AND revoked_at IS NULL",
    )
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}

pub async fn insert_password_reset_token(
    db: &sqlx::PgPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at)
         VALUES ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn invalidate_password_reset_tokens(db: &sqlx::PgPool, user_id: Uuid) -> ApiResult<()> {
    sqlx::query(
        "UPDATE password_reset_tokens
            SET used_at = now()
          WHERE user_id = $1
            AND used_at IS NULL",
    )
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn find_user_for_active_password_reset_token(
    db: &sqlx::PgPool,
    token_hash: &str,
) -> ApiResult<Option<Uuid>> {
    let user_id: Option<(Uuid,)> = sqlx::query_as(
        "SELECT user_id
           FROM password_reset_tokens
          WHERE token_hash = $1
            AND used_at IS NULL
            AND expires_at > now()",
    )
    .bind(token_hash)
    .fetch_optional(db)
    .await?;

    Ok(user_id.map(|(id,)| id))
}

pub async fn consume_password_reset_token(
    db: &sqlx::PgPool,
    token_hash: &str,
) -> ApiResult<Option<Uuid>> {
    let user_id: Option<(Uuid,)> = sqlx::query_as(
        "UPDATE password_reset_tokens
            SET used_at = now()
          WHERE token_hash = $1
            AND used_at IS NULL
            AND expires_at > now()
      RETURNING user_id",
    )
    .bind(token_hash)
    .fetch_optional(db)
    .await?;

    Ok(user_id.map(|(id,)| id))
}

pub async fn count_recent_password_reset_requests(
    db: &sqlx::PgPool,
    user_id: Uuid,
    since: DateTime<Utc>,
) -> ApiResult<i64> {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM password_reset_tokens WHERE user_id = $1 AND created_at > $2",
    )
    .bind(user_id)
    .bind(since)
    .fetch_one(db)
    .await?;

    Ok(count)
}
