mod create_location;
mod delete_location;
mod get_location;
mod list_locations;
mod models;
pub mod repository;
mod update_location;

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;

pub use models::Location;

/// OpenAPI tag every endpoint in this module is filed under.
pub const TAG: &str = "locations";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_locations::handler, create_location::handler))
        .routes(routes!(
            get_location::handler,
            update_location::handler,
            delete_location::handler
        ))
}
