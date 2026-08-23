//! The API as a library, so that both the binary and the test suite can build
//! the very same router and application state.
//!
//! `main.rs` is a thin wrapper around this: it reads the environment, opens the
//! pool and serves [`endpoints::build`]. The BDD suite under `tests/` does the
//! same against a throwaway database, which is only possible because everything
//! below is reachable from outside the crate.

pub mod config;
pub mod endpoints;
pub mod error;
pub mod mail;
pub mod routing;

use std::sync::Arc;

use crate::config::Config;
use crate::endpoints::users::LoginProvider;
use crate::routing::RoutingProvider;

/// Everything a handler is given access to, cloned into each request.
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: Config,
    pub login: Arc<dyn LoginProvider>,
    pub routing: Arc<dyn RoutingProvider>,
}
