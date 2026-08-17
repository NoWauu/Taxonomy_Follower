use axum::{Router, routing::get};
use sqlx::postgres::PgPoolOptions;



#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    let app = Router::new()
    .route("/health", get(health));

    let listener =
        tokio::net::TcpListener::bind("0.0.0.0:3000")
            .await?;

    
    // Hangs here, so Ok is never reached
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "OK"
}