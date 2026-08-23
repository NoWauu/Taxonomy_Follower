use std::sync::Arc;

use axum::http::{Method, header};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;

use api::config::Config;
use api::endpoints::users::LocalLoginProvider;
use api::{AppState, endpoints, mail, routing};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Emitting the spec must work without a database or any environment, so it
    // is handled before anything else is set up.
    if std::env::args().any(|arg| arg == "--dump-openapi") {
        println!("{}", endpoints::openapi().to_pretty_json()?);
        return Ok(());
    }

    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Config::from_env()?;
    let mailer = mail::from_config(&config.mail)?;
    let routing = routing::from_config(&config.routing)?;

    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    let login = Arc::new(LocalLoginProvider::new(
        pool.clone(),
        config.clone(),
        mailer,
    ));

    let cors = CorsLayer::new()
        .allow_origin(config.cors_allowed_origins.clone())
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    let port = config.port;
    let public_url = config.public_url.clone();

    let state = AppState {
        db: pool,
        config,
        login,
        routing,
    };

    let (router, _api) = endpoints::build(Some(&public_url));
    let app = router.layer(cors).with_state(state);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    tracing::info!(
        "OpenAPI document at http://{}{}",
        listener.local_addr()?,
        endpoints::OPENAPI_PATH
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;

    Ok(())
}
