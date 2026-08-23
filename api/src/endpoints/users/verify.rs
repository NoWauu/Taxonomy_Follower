use axum::Json;
use axum::extract::State;
use validator::Validate;

use crate::AppState;
use crate::error::{ApiResult, ErrorResponse};

use super::extractor::AuthenticatedUser;
use super::local::strip_bearer_prefix;
use super::models::{User, VerifyTokenRequest, VerifyTokenResponse};

/// Check whether an access token is still valid
///
/// The token may be submitted bare or with a leading `Bearer ` prefix.
#[utoipa::path(
    post,
    path = "/token/verify",
    tag = super::TAG,
    request_body = VerifyTokenRequest,
    responses(
        (status = OK, description = "Token is valid", body = VerifyTokenResponse),
        (status = BAD_REQUEST, description = "Payload failed validation", body = ErrorResponse),
        (status = UNAUTHORIZED, description = "Token is invalid or expired", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    Json(payload): Json<VerifyTokenRequest>,
) -> ApiResult<Json<VerifyTokenResponse>> {
    payload.validate()?;

    let submitted = payload.token.trim();
    let access_token = strip_bearer_prefix(submitted).unwrap_or(submitted);

    let identity = state.login.verify_access_token(access_token).await?;

    Ok(Json(VerifyTokenResponse {
        valid: true,
        user_id: identity.user.id,
        email: identity.user.email,
        issued_at: identity.issued_at,
        expires_at: identity.expires_at,
    }))
}

/// Return the currently authenticated user
#[utoipa::path(
    get,
    path = "/me",
    tag = super::TAG,
    security(("bearer_auth" = [])),
    responses(
        (status = OK, description = "The authenticated user", body = User),
        (status = UNAUTHORIZED, description = "Missing or invalid access token", body = ErrorResponse),
    ),
)]
pub async fn current_user(AuthenticatedUser(user): AuthenticatedUser) -> ApiResult<Json<User>> {
    Ok(Json(user))
}
