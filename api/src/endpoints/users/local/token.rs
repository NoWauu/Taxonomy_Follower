use anyhow::Context;
use argon2::password_hash::rand_core::{OsRng, RngCore};
use chrono::{DateTime, TimeZone, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::Config;
use crate::error::{ApiError, ApiResult};

use super::super::models::UserRecord;

pub const TOKEN_TYPE: &str = "Bearer";

const SIGNING_ALGORITHM: Algorithm = Algorithm::HS256;
const OPAQUE_TOKEN_BYTES: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: Uuid,
}

impl Claims {
    pub fn issued_at(&self) -> DateTime<Utc> {
        to_datetime(self.iat)
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        to_datetime(self.exp)
    }
}

fn to_datetime(unix_seconds: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(unix_seconds, 0)
        .single()
        .unwrap_or_default()
}

pub fn sign_access_token(config: &Config, user: &UserRecord) -> ApiResult<(String, DateTime<Utc>)> {
    let issued_at = Utc::now();
    let lifetime = chrono::Duration::from_std(config.access_token_ttl)
        .context("access token TTL does not fit in a chrono duration")?;
    let expires_at = issued_at + lifetime;

    let claims = Claims {
        sub: user.id,
        email: user.email.clone(),
        iat: issued_at.timestamp(),
        exp: expires_at.timestamp(),
        jti: Uuid::new_v4(),
    };

    let access_token = encode(
        &Header::new(SIGNING_ALGORITHM),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .context("failed to sign the access token")?;

    Ok((access_token, expires_at))
}

pub fn decode_access_token(config: &Config, access_token: &str) -> ApiResult<Claims> {
    let mut validation = Validation::new(SIGNING_ALGORITHM);
    validation.validate_exp = true;
    validation.required_spec_claims = ["exp", "iat", "sub"]
        .into_iter()
        .map(String::from)
        .collect();

    decode::<Claims>(
        access_token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &validation,
    )
    .map(|token_data| token_data.claims)
    .map_err(|error| {
        tracing::debug!(%error, "access token rejected");
        match error.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                ApiError::Unauthorized("access token has expired")
            }
            _ => ApiError::Unauthorized("invalid access token"),
        }
    })
}

pub fn strip_bearer_prefix(header_value: &str) -> Option<&str> {
    let (scheme, token) = header_value.split_once(' ')?;

    scheme
        .eq_ignore_ascii_case(TOKEN_TYPE)
        .then(|| token.trim())
        .filter(|token| !token.is_empty())
}

pub struct OpaqueToken {
    pub raw: String,
    pub hash: String,
}

pub fn generate_opaque_token() -> OpaqueToken {
    let mut entropy = [0u8; OPAQUE_TOKEN_BYTES];
    OsRng.fill_bytes(&mut entropy);
    let raw = hex::encode(entropy);
    let hash = hash_opaque_token(&raw);

    OpaqueToken { raw, hash }
}

pub fn hash_opaque_token(raw: &str) -> String {
    hex::encode(Sha256::digest(raw.as_bytes()))
}
