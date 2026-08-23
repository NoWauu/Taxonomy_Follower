mod create_trip;
mod delete_trip;
mod get_trip;
mod list_trips;
mod models;
mod repository;
mod update_trip;
mod validation;

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;

/// OpenAPI tag every endpoint in this module is filed under.
pub const TAG: &str = "trips";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_trips::handler, create_trip::handler))
        .routes(routes!(
            get_trip::handler,
            update_trip::handler,
            delete_trip::handler
        ))
}
