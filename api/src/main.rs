use axum::{Router, routing::get};
use sqlx::postgres::PgPoolOptions;

use crate::endpoints::health;

mod endpoints;

#[derive(Clone)]
struct AppState {
    db: sqlx::PgPool
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    let state = AppState {
        db: pool
    };

    let app = Router::new()
    .route("/health", get(health))
    .with_state(state);

    let listener =
        tokio::net::TcpListener::bind("0.0.0.0:3000")
            .await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.ok(); })
        .await?;

    Ok(())
}