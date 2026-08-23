mod extractor;
mod forgot_password;
mod local;
mod login;
mod logout;
mod models;
mod provider;
mod refresh;
mod register;
mod reset_password;
mod verify;

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;

pub use extractor::AuthenticatedUser;
pub use local::LocalLoginProvider;
pub use provider::LoginProvider;

/// OpenAPI tag every endpoint in this module is filed under.
pub const TAG: &str = "users";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(register::handler))
        .routes(routes!(login::handler))
        .routes(routes!(logout::handler))
        .routes(routes!(logout::everywhere))
        .routes(routes!(refresh::handler))
        .routes(routes!(verify::handler))
        .routes(routes!(forgot_password::handler))
        .routes(routes!(reset_password::handler))
        .routes(routes!(verify::current_user))
}
