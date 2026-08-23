use axum::Json;
use axum::extract::State;
use validator::Validate;

use crate::AppState;
use crate::error::{ApiResult, ErrorResponse};

use super::models::{RefreshRequest, TokenPair};

/// Exchange a refresh token for a new token pair
///
/// The submitted refresh token is rotated: it is revoked and a new one is
/// returned alongside the new access token.
#[utoipa::path(
    post,
    path = "/token/refresh",
    tag = super::TAG,
    request_body = RefreshRequest,
    responses(
        (status = OK, description = "New token pair issued", body = TokenPair),
        (status = BAD_REQUEST, description = "Payload failed validation", body = ErrorResponse),
        (status = UNAUTHORIZED, description = "Refresh token is unknown, expired or revoked", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> ApiResult<Json<TokenPair>> {
    payload.validate()?;

    let tokens = state.login.refresh(&payload.refresh_token).await?;

    Ok(Json(tokens))
}
