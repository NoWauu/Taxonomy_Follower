use axum::Json;
use axum::extract::State;
use validator::Validate;

use crate::AppState;
use crate::error::{ApiResult, ErrorResponse};

use super::models::{LoginRequest, Session};
use super::provider::Credentials;

/// Sign in with email and password
#[utoipa::path(
    post,
    path = "/login",
    operation_id = "login",
    tag = super::TAG,
    request_body = LoginRequest,
    responses(
        (status = OK, description = "Signed in", body = Session),
        (status = BAD_REQUEST, description = "Payload failed validation", body = ErrorResponse),
        (status = UNAUTHORIZED, description = "Unknown email or wrong password", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> ApiResult<Json<Session>> {
    payload.validate()?;

    let session = state
        .login
        .login(Credentials {
            email: payload.email,
            password: payload.password,
        })
        .await?;

    Ok(Json(session))
}
