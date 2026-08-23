use std::time::Duration;

use anyhow::Context;
use axum::http::HeaderValue;

use crate::routing::RouteProfile;

#[derive(Clone, Debug)]
pub struct Config {
    /// TCP port the API listens on.
    pub port: u16,
    pub jwt_secret: String,
    pub access_token_ttl: Duration,
    pub refresh_token_ttl: Duration,
    pub password_reset_ttl: Duration,
    pub app_url: String,
    /// Base URL the API is reachable at from a browser. Ends up as the
    /// `servers` entry of the OpenAPI document, so "Try it out" in Swagger UI
    /// aims at the right host.
    pub public_url: String,
    pub mail: MailConfig,
    pub routing: RoutingConfig,
    /// Browser origins allowed to call the API, e.g. the Next.js dev server.
    pub cors_allowed_origins: Vec<HeaderValue>,
}

#[derive(Clone, Debug)]
pub struct MailConfig {
    pub smtp: Option<SmtpConfig>,
    pub sender: String,
}

#[derive(Clone, Debug)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub encryption: SmtpEncryption,
}

/// How trip itineraries are computed.
#[derive(Clone, Debug)]
pub struct RoutingConfig {
    /// `None` disables real routing: distances fall back to straight lines.
    pub mapbox: Option<MapboxConfig>,
    /// Profile used when the caller does not pick one.
    pub default_profile: RouteProfile,
    /// How long a single routing call may take before it is given up on.
    pub timeout: Duration,
}

#[derive(Clone, Debug)]
pub struct MapboxConfig {
    pub access_token: String,
    /// Root the Directions API is called on. Overridable so tests can aim at a
    /// local stub.
    pub base_url: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmtpEncryption {
    Tls,
    StartTls,
    None,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let jwt_secret = required("JWT_SECRET")?;
        anyhow::ensure!(
            jwt_secret.len() >= 32,
            "JWT_SECRET must be at least 32 characters long"
        );

        let port = match optional("API_PORT") {
            Some(raw) => raw.parse().context("API_PORT must be a valid port")?,
            None => 8080,
        };

        Ok(Self {
            port,
            jwt_secret,
            access_token_ttl: duration_secs("ACCESS_TOKEN_TTL_SECONDS", 15 * 60)?,
            refresh_token_ttl: duration_secs("REFRESH_TOKEN_TTL_SECONDS", 30 * 24 * 60 * 60)?,
            password_reset_ttl: duration_secs("PASSWORD_RESET_TTL_SECONDS", 60 * 60)?,
            app_url: optional("APP_URL")
                .unwrap_or_else(|| "http://localhost:3000".to_string())
                .trim_end_matches('/')
                .to_string(),
            public_url: optional("PUBLIC_API_URL")
                .unwrap_or_else(|| format!("http://localhost:{port}"))
                .trim_end_matches('/')
                .to_string(),
            mail: MailConfig::from_env()?,
            routing: RoutingConfig::from_env()?,
            cors_allowed_origins: cors_allowed_origins()?,
        })
    }
}

impl MailConfig {
    fn from_env() -> anyhow::Result<Self> {
        let sender = optional("SMTP_FROM")
            .unwrap_or_else(|| "Taxonomy Follower <no-reply@localhost>".to_string());

        let Some(host) = optional("SMTP_HOST") else {
            return Ok(Self { smtp: None, sender });
        };

        let encryption = match optional("SMTP_ENCRYPTION")
            .unwrap_or_else(|| "starttls".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "tls" | "ssl" | "implicit" => SmtpEncryption::Tls,
            "starttls" => SmtpEncryption::StartTls,
            "none" | "plain" | "" => SmtpEncryption::None,
            other => anyhow::bail!(
                "invalid SMTP_ENCRYPTION `{other}`, expected one of: tls, starttls, none"
            ),
        };

        let default_port = match encryption {
            SmtpEncryption::Tls => 465,
            SmtpEncryption::StartTls => 587,
            SmtpEncryption::None => 25,
        };

        let port = match optional("SMTP_PORT") {
            Some(raw) => raw.parse().context("SMTP_PORT must be a valid port")?,
            None => default_port,
        };

        Ok(Self {
            smtp: Some(SmtpConfig {
                host,
                port,
                username: optional("SMTP_USERNAME"),
                password: optional("SMTP_PASSWORD"),
                encryption,
            }),
            sender,
        })
    }
}

impl RoutingConfig {
    fn from_env() -> anyhow::Result<Self> {
        let default_profile = match optional("ROUTING_PROFILE") {
            Some(raw) => raw.parse()?,
            None => RouteProfile::default(),
        };

        let mapbox = optional("MAPBOX_ACCESS_TOKEN").map(|access_token| MapboxConfig {
            access_token,
            base_url: optional("MAPBOX_BASE_URL")
                .unwrap_or_else(|| "https://api.mapbox.com".to_string())
                .trim_end_matches('/')
                .to_string(),
        });

        Ok(Self {
            mapbox,
            default_profile,
            timeout: duration_secs("ROUTING_TIMEOUT_SECONDS", 10)?,
        })
    }
}

/// Origins allowed by CORS.
///
/// Defaults to `APP_URL` alone; set `CORS_ALLOWED_ORIGINS` to a comma separated
/// list to widen it. Wildcards are deliberately not supported, since the API
/// answers with credentials-bearing tokens.
fn cors_allowed_origins() -> anyhow::Result<Vec<HeaderValue>> {
    let raw = optional("CORS_ALLOWED_ORIGINS")
        .or_else(|| optional("APP_URL"))
        .unwrap_or_else(|| "http://localhost:3000".to_string());

    raw.split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(|origin| {
            HeaderValue::from_str(origin.trim_end_matches('/'))
                .with_context(|| format!("`{origin}` is not a valid CORS origin"))
        })
        .collect()
}

fn optional(key: &str) -> Option<String> {
    match std::env::var(key) {
        Ok(value) if !value.trim().is_empty() => Some(value.trim().to_string()),
        _ => None,
    }
}

fn required(key: &str) -> anyhow::Result<String> {
    optional(key).with_context(|| format!("missing required environment variable `{key}`"))
}

fn duration_secs(key: &str, default: u64) -> anyhow::Result<Duration> {
    let secs = match optional(key) {
        Some(raw) => raw
            .parse::<u64>()
            .with_context(|| format!("`{key}` must be a number of seconds"))?,
        None => default,
    };
    anyhow::ensure!(secs > 0, "`{key}` must be greater than zero");
    Ok(Duration::from_secs(secs))
}
