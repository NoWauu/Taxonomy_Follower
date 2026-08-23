use axum::Router;
use axum::routing::get;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;

mod health;
pub mod users;

/// OpenAPI tag for the health probe.
pub const HEALTH_TAG: &str = "health";

/// Path the generated OpenAPI document is served from.
pub const OPENAPI_PATH: &str = "/openapi.json";

/// Registers the `bearer_auth` security scheme referenced by the protected paths.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi
            .components
            .get_or_insert_with(utoipa::openapi::Components::default);

        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some("Access token issued by `/users/login`."))
                    .build(),
            ),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    info(
        title = "Taxonomy Follower API",
        description = "HTTP API backing the Taxonomy Follower web app.",
        license(name = "MIT"),
    ),
    servers((url = "http://localhost:8080", description = "Local development")),
    tags(
        (name = HEALTH_TAG, description = "Service and dependency probes"),
        (name = users::TAG, description = "Accounts, sessions and password recovery"),
    ),
)]
pub struct ApiDoc;

/// Builds the axum router together with the OpenAPI document describing it.
///
/// Both come out of the same `OpenApiRouter`, so a route can never drift away
/// from its documentation. `public_url`, when given, replaces the default
/// `servers` entry of the document.
pub fn build(public_url: Option<&str>) -> (Router<AppState>, utoipa::openapi::OpenApi) {
    let mut doc = ApiDoc::openapi();
    if let Some(url) = public_url {
        doc.servers = Some(vec![
            utoipa::openapi::ServerBuilder::new()
                .url(url)
                .description(Some("This API"))
                .build(),
        ]);
    }

    let (router, api) = OpenApiRouter::with_openapi(doc)
        .routes(routes!(health::handler))
        .nest("/users", users::router())
        .split_for_parts();

    let spec = api.clone();
    let router = router.route(
        OPENAPI_PATH,
        get(move || {
            let spec = spec.clone();
            async move { axum::Json(spec) }
        }),
    );

    (router, api)
}

/// The OpenAPI document on its own, for `--dump-openapi`.
pub fn openapi() -> utoipa::openapi::OpenApi {
    build(None).1
}
