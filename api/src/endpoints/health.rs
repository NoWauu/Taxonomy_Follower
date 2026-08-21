use axum::{extract::State, http::StatusCode};

use crate::AppState;

pub async fn main(
    State(state): State<AppState>,
) -> Result<&'static str, (StatusCode, &'static str)> {
    sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Health check failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "database unavailable")
        })?;

    Ok("OK")
}