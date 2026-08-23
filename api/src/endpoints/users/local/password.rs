use std::collections::HashSet;
use std::sync::LazyLock;

use anyhow::Context;
use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

const MIN_DISTINCT_CHARACTERS: usize = 5;

static UNMATCHABLE_HASH: LazyLock<String> = LazyLock::new(|| {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(Uuid::new_v4().as_bytes(), &salt)
        .expect("hashing a random value with default parameters cannot fail")
        .to_string()
});

pub async fn hash(password: String) -> ApiResult<String> {
    let hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| anyhow::anyhow!("failed to hash password: {error}"))
    })
    .await
    .context("password hashing task panicked")??;

    Ok(hash)
}

pub async fn matches(password: String, expected_hash: String) -> ApiResult<bool> {
    let matches = tokio::task::spawn_blocking(move || {
        let parsed_hash = match PasswordHash::new(&expected_hash) {
            Ok(parsed_hash) => parsed_hash,
            Err(error) => {
                tracing::error!(%error, "stored password hash is malformed");
                return false;
            }
        };

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    })
    .await
    .context("password verification task panicked")?;

    Ok(matches)
}

pub async fn spend_time_as_if_verifying(password: String) {
    if let Err(error) = matches(password, UNMATCHABLE_HASH.clone()).await {
        tracing::debug!(?error, "timing-equalising verification failed");
    }
}

pub fn reject_if_predictable(password: &str, email: &str) -> ApiResult<()> {
    let lowercased_password = password.to_lowercase();
    let email_local_part = email.split('@').next().unwrap_or_default().to_lowercase();

    let contains_email =
        !email_local_part.is_empty() && lowercased_password.contains(&email_local_part);

    if contains_email {
        return Err(ApiError::BadRequest(
            "password must not contain your email address".into(),
        ));
    }

    if password.chars().collect::<HashSet<_>>().len() < MIN_DISTINCT_CHARACTERS {
        return Err(ApiError::BadRequest(
            "password must use at least 5 different characters".into(),
        ));
    }

    Ok(())
}
