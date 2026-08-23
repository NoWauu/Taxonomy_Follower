use axum::{extract::State, http::StatusCode};

use crate::AppState;

/// Liveness and database connectivity probe
#[utoipa::path(
    get,
    path = "/health",
    operation_id = "healthCheck",
    tag = super::HEALTH_TAG,
    responses(
        (status = OK, description = "API and database are reachable", body = String, example = json!("OK")),
        (status = INTERNAL_SERVER_ERROR, description = "The database could not be reached", body = String),
    ),
)]
pub async fn handler(
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
