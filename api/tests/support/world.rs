//! The state a scenario carries from one step to the next.
//!
//! Every scenario gets a freshly emptied database and a router built exactly
//! the way `main.rs` builds it, minus the socket: requests are handed to the
//! `Router` directly through `tower`, so nothing binds a port and nothing is
//! left running between scenarios.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use api::AppState;
use api::config::{Config, MailConfig, MapboxConfig, RoutingConfig};
use api::endpoints::users::LocalLoginProvider;
use api::routing::{Route, RouteProfile, RoutingProvider};
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use chrono::{DateTime, Utc};
use http_body_util::BodyExt as _;
use serde_json::Value;
use sqlx::AssertSqlSafe;
use sqlx::{PgPool, Row as _};
use tokio::sync::OnceCell;
use tower::ServiceExt as _;
use uuid::Uuid;

use super::mail::CapturingMailProvider;
use super::mapbox_stub::MapboxStub;

/// Password every fixture account is created with. Long enough for the twelve
/// character minimum and varied enough for the "five distinct characters" rule.
pub const FIXTURE_PASSWORD: &str = "correct-horse-battery-42";

/// Tables emptied between scenarios, children first so the foreign keys hold.
const TABLES: &str = "trip_stops, trips, locations, password_reset_tokens, refresh_tokens, users";

static POOL: OnceCell<PgPool> = OnceCell::const_new();

/// An account created by a `Given`, with the tokens it was handed.
#[derive(Debug, Clone)]
pub struct TestUser {
    pub id: Uuid,
    pub password: String,
    pub access_token: String,
    pub refresh_token: String,
}

/// What the last request answered.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub status: StatusCode,
    pub body: Option<Value>,
    pub raw: String,
}

#[derive(cucumber::World)]
#[world(init = Self::new)]
pub struct World {
    pub db: PgPool,
    pub router: Router,
    pub mails: Arc<CapturingMailProvider>,
    pub mapbox: MapboxStub,

    pub users: HashMap<String, TestUser>,
    pub locations: HashMap<String, i32>,
    /// Coordinates of each named location, for the routing steps, which work on
    /// points rather than on rows.
    pub coordinates: HashMap<String, (f64, f64)>,
    pub trips: HashMap<String, i32>,

    /// Whose bearer token authenticated requests carry.
    pub current_user: Option<String>,
    /// Reset token pulled out of the last password reset email.
    pub reset_token: Option<String>,
    pub response: Option<Recorded>,

    /// Result of the last direct call to a routing provider. Routing has no
    /// endpoint yet, so those scenarios drive the provider itself.
    pub route: Option<Result<Route, String>>,
    pub routing: Arc<dyn RoutingProvider>,
}

impl fmt::Debug for World {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("World")
            .field("current_user", &self.current_user)
            .field("users", &self.users.keys().collect::<Vec<_>>())
            .field("locations", &self.locations)
            .field("trips", &self.trips)
            .field("response", &self.response)
            .field("route", &self.route)
            .finish_non_exhaustive()
    }
}

impl World {
    async fn new() -> anyhow::Result<Self> {
        let db = POOL
            .get_or_try_init(prepare_database)
            .await
            .context("the BDD suite needs a reachable Postgres with PostGIS; `docker compose up -d database` starts one")?
            .clone();

        sqlx::query(AssertSqlSafe(format!(
            "TRUNCATE {TABLES} RESTART IDENTITY CASCADE"
        )))
        .execute(&db)
        .await
        .context("failed to empty the test database")?;

        let mails = CapturingMailProvider::new();
        let mapbox = MapboxStub::start().await?;

        let config = test_config();
        let login = Arc::new(LocalLoginProvider::new(
            db.clone(),
            config.clone(),
            Arc::clone(&mails) as Arc<_>,
        ));
        let routing = api::routing::from_config(&config.routing)?;

        let state = AppState {
            db: db.clone(),
            config,
            login,
            routing: Arc::clone(&routing),
        };

        let (router, _) = api::endpoints::build(None);

        Ok(Self {
            db,
            router: router.with_state(state),
            mails,
            mapbox,
            users: HashMap::new(),
            locations: HashMap::new(),
            coordinates: HashMap::new(),
            trips: HashMap::new(),
            current_user: None,
            reset_token: None,
            response: None,
            route: None,
            routing,
        })
    }

    // -- HTTP ---------------------------------------------------------------

    /// Sends a request through the router and records what came back.
    ///
    /// `authenticated` decides whether the current user's access token is
    /// attached; scenarios about missing credentials pass `false`.
    pub async fn send(
        &mut self,
        method: Method,
        path: &str,
        body: Option<String>,
        authenticated: bool,
    ) {
        let path = self.resolve(path);
        let mut request = Request::builder().method(method).uri(&path);

        if authenticated && let Some(token) = self.access_token() {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }

        let request = match body {
            Some(body) => request
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(self.resolve(&body)))
                .expect("the request is well formed"),
            None => request
                .body(Body::empty())
                .expect("the request is well formed"),
        };

        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("the router answers every request");

        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("the response body is readable")
            .to_bytes();
        let raw = String::from_utf8_lossy(&bytes).to_string();

        self.response = Some(Recorded {
            status,
            body: serde_json::from_str(&raw).ok(),
            raw,
        });
    }

    pub fn last(&self) -> &Recorded {
        self.response
            .as_ref()
            .expect("a step must send a request before asserting on the response")
    }

    /// Reads a dotted path out of the last response body, e.g. `user.email` or
    /// `0.stops.1.location.name`.
    pub fn field(&self, path: &str) -> Option<&Value> {
        let mut current = self.last().body.as_ref()?;

        for segment in path.split('.') {
            current = match current {
                Value::Array(items) => items.get(segment.parse::<usize>().ok()?)?,
                _ => current.get(segment)?,
            };
        }

        Some(current)
    }

    fn access_token(&self) -> Option<String> {
        let alias = self.current_user.as_ref()?;
        Some(self.users.get(alias)?.access_token.clone())
    }

    pub fn user(&self, alias: &str) -> &TestUser {
        self.users
            .get(alias)
            .unwrap_or_else(|| panic!("no registered user `{alias}` in this scenario"))
    }

    pub fn location_id(&self, name: &str) -> i32 {
        *self
            .locations
            .get(name)
            .unwrap_or_else(|| panic!("no location `{name}` in this scenario"))
    }

    pub fn trip_id(&self, name: &str) -> i32 {
        *self
            .trips
            .get(name)
            .unwrap_or_else(|| panic!("no trip `{name}` in this scenario"))
    }

    // -- Fixtures -----------------------------------------------------------

    /// Registers an account through the API, so the tokens are real ones.
    pub async fn register(&mut self, email: &str, password: &str) -> anyhow::Result<()> {
        let body = serde_json::json!({ "email": email, "password": password }).to_string();
        self.send(Method::POST, "/users/register", Some(body), false)
            .await;

        let recorded = self.last().clone();
        anyhow::ensure!(
            recorded.status == StatusCode::CREATED,
            "registering `{email}` failed with {}: {}",
            recorded.status,
            recorded.raw
        );

        let body = recorded.body.expect("register answers with a session");
        self.users.insert(
            email.to_string(),
            TestUser {
                id: body["user"]["id"]
                    .as_str()
                    .and_then(|id| id.parse().ok())
                    .context("register answered without a user id")?,
                password: password.to_string(),
                access_token: body["access_token"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                refresh_token: body["refresh_token"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            },
        );

        Ok(())
    }

    /// Inserts a location straight into the database.
    ///
    /// Fixtures bypass the endpoints on purpose: a scenario about trips should
    /// not fail because location creation broke, and `locations.feature` covers
    /// that endpoint on its own.
    pub async fn insert_location(
        &mut self,
        name: &str,
        latitude: f64,
        longitude: f64,
    ) -> anyhow::Result<i32> {
        let id: i32 = sqlx::query_scalar(
            r#"INSERT INTO locations (name, position)
               VALUES ($1, ST_SetSRID(ST_MakePoint($2, $3), 4326)::geography)
               RETURNING id"#,
        )
        .bind(name)
        .bind(longitude)
        .bind(latitude)
        .fetch_one(&self.db)
        .await?;

        self.locations.insert(name.to_string(), id);
        self.coordinates
            .insert(name.to_string(), (latitude, longitude));

        Ok(id)
    }

    pub async fn insert_trip(
        &mut self,
        name: &str,
        owner: Uuid,
        start_date: DateTime<Utc>,
        start_location_id: i32,
        end_location_id: i32,
        available_seats: i32,
    ) -> anyhow::Result<i32> {
        let id: i32 = sqlx::query_scalar(
            r#"INSERT INTO trips (created_by, start_date, start_location_id, end_location_id, available_seats)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id"#,
        )
        .bind(owner)
        .bind(start_date)
        .bind(start_location_id)
        .bind(end_location_id)
        .bind(available_seats)
        .fetch_one(&self.db)
        .await?;

        self.trips.insert(name.to_string(), id);

        Ok(id)
    }

    pub async fn insert_stop(
        &self,
        trip_id: i32,
        location_id: i32,
        stop_order: i32,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO trip_stops (trip_id, stop_location_id, stop_order) VALUES ($1, $2, $3)",
        )
        .bind(trip_id)
        .bind(location_id)
        .bind(stop_order)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn count(&self, table: &str) -> i64 {
        sqlx::query(AssertSqlSafe(format!("SELECT count(*) AS n FROM {table}")))
            .fetch_one(&self.db)
            .await
            .expect("counting rows works")
            .get::<i64, _>("n")
    }

    pub fn coordinates_of(&self, name: &str) -> (f64, f64) {
        *self
            .coordinates
            .get(name)
            .unwrap_or_else(|| panic!("no coordinates recorded for `{name}`"))
    }

    // -- Routing ------------------------------------------------------------

    /// Points the routing steps at a Mapbox client aimed at the local stub.
    pub fn use_mapbox(&mut self) {
        let mapbox = MapboxConfig {
            access_token: "test-token".to_string(),
            base_url: self.mapbox.base_url(),
        };

        self.routing = Arc::new(
            api::routing::MapboxRoutingProvider::new(&mapbox, Duration::from_secs(5))
                .expect("the Mapbox client is buildable"),
        );
    }

    /// Goes back to straight-line estimates, the provider used when no token is
    /// configured.
    pub fn use_fallback_routing(&mut self) {
        self.routing = Arc::new(api::routing::HaversineRoutingProvider);
    }

    // -- Templating ---------------------------------------------------------

    /// Expands the `{kind:argument}` placeholders a feature file may use inside
    /// a path or a JSON body, so scenarios can name things instead of guessing
    /// the identifiers a previous step created.
    ///
    /// Anything that is not a known placeholder — a JSON object, most
    /// obviously — is left untouched.
    pub fn resolve(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;

        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let after = &rest[open + 1..];

            let Some(close) = after.find('}') else {
                out.push('{');
                rest = after;
                continue;
            };

            match self.placeholder(&after[..close]) {
                Some(value) => {
                    out.push_str(&value);
                    rest = &after[close + 1..];
                }
                None => {
                    out.push('{');
                    rest = after;
                }
            }
        }

        out.push_str(rest);
        out
    }

    fn placeholder(&self, inner: &str) -> Option<String> {
        let (kind, argument) = inner.split_once(':')?;

        match kind {
            "location" => Some(self.locations.get(argument)?.to_string()),
            "trip" => Some(self.trips.get(argument)?.to_string()),
            "user" => Some(self.users.get(argument)?.id.to_string()),
            "token" => match argument {
                "access" => self.access_token(),
                "refresh" => Some(
                    self.users
                        .get(self.current_user.as_ref()?)?
                        .refresh_token
                        .clone(),
                ),
                "reset" => self.reset_token.clone(),
                _ => None,
            },
            "in" | "ago" => {
                let amount = shift(argument)?;
                let instant = if kind == "in" {
                    Utc::now() + amount
                } else {
                    Utc::now() - amount
                };
                // `Z` rather than `+00:00`: these dates end up in query
                // strings, where a plus sign would decode as a space.
                Some(instant.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
            }
            _ => None,
        }
    }
}

/// Parses `3 days`, `1 hour`, `30 minutes` into a duration.
fn shift(argument: &str) -> Option<chrono::Duration> {
    let (amount, unit) = argument.trim().split_once(' ')?;
    let amount: i64 = amount.parse().ok()?;

    match unit.trim_end_matches('s') {
        "day" => Some(chrono::Duration::days(amount)),
        "hour" => Some(chrono::Duration::hours(amount)),
        "minute" => Some(chrono::Duration::minutes(amount)),
        "second" => Some(chrono::Duration::seconds(amount)),
        _ => None,
    }
}

/// Configuration the suite runs against. Built by hand rather than read from
/// the environment, so a stray `.env` cannot change what the tests assert.
fn test_config() -> Config {
    Config {
        port: 0,
        jwt_secret: "bdd-suite-signing-secret-at-least-32-chars".to_string(),
        access_token_ttl: Duration::from_secs(900),
        refresh_token_ttl: Duration::from_secs(3600),
        password_reset_ttl: Duration::from_secs(3600),
        app_url: "http://localhost:3000".to_string(),
        public_url: "http://localhost:8080".to_string(),
        mail: MailConfig {
            smtp: None,
            sender: "Taxonomy Follower <no-reply@localhost>".to_string(),
        },
        routing: RoutingConfig {
            mapbox: None,
            default_profile: RouteProfile::Driving,
            timeout: Duration::from_secs(5),
        },
        cors_allowed_origins: Vec::new(),
    }
}

/// Opens (creating it if needed) the database the suite owns, and migrates it.
///
/// It is deliberately not the development database: `TEST_DATABASE_URL` wins,
/// otherwise `DATABASE_URL` is reused with `_test` appended to its name.
async fn prepare_database() -> anyhow::Result<PgPool> {
    let url = test_database_url()?;

    if let Err(error) = PgPool::connect(&url).await {
        tracing_hint(&error);
        create_database(&url).await?;
    }

    let pool = PgPool::connect(&url)
        .await
        .with_context(|| format!("failed to connect to the test database at {url}"))?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .context("failed to migrate the test database")?;

    Ok(pool)
}

fn test_database_url() -> anyhow::Result<String> {
    dotenvy::dotenv().ok();

    if let Ok(url) = std::env::var("TEST_DATABASE_URL") {
        return Ok(url);
    }

    let development = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5432/taxonomy_follower".to_string()
    });

    let (server, database) = development
        .rsplit_once('/')
        .context("DATABASE_URL has no database name")?;
    let (database, query) = match database.split_once('?') {
        Some((database, query)) => (database, format!("?{query}")),
        None => (database, String::new()),
    };

    Ok(format!("{server}/{database}_test{query}"))
}

/// Creates the test database from the `postgres` maintenance database.
async fn create_database(url: &str) -> anyhow::Result<()> {
    let (server, database) = url.rsplit_once('/').context("URL has no database name")?;
    let database = database.split('?').next().unwrap_or(database);

    let maintenance = PgPool::connect(&format!("{server}/postgres"))
        .await
        .with_context(|| format!("failed to reach the Postgres server at {server}"))?;

    sqlx::query(AssertSqlSafe(format!(r#"CREATE DATABASE "{database}""#)))
        .execute(&maintenance)
        .await
        .with_context(|| format!("failed to create the test database `{database}`"))?;

    maintenance.close().await;

    Ok(())
}

fn tracing_hint(error: &sqlx::Error) {
    eprintln!("test database not reachable yet ({error}), trying to create it");
}
