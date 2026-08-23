use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;

use crate::AppState;
use crate::error::ApiError;

use super::local::strip_bearer_prefix;
use super::models::User;

pub struct AuthenticatedUser(pub User);

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header_value = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(ApiError::Unauthorized("missing Authorization header"))?;

        let access_token = strip_bearer_prefix(header_value).ok_or(ApiError::Unauthorized(
            "expected an `Authorization: Bearer <token>` header",
        ))?;

        let identity = state.login.verify_access_token(access_token).await?;

        Ok(Self(identity.user))
    }
}
