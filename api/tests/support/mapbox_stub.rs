//! A stand-in for the Mapbox Directions API.
//!
//! It answers every path with whatever the running scenario last handed it,
//! which is enough to drive the real HTTP client through its success and
//! failure branches without an account or a network.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::http::{Response, StatusCode};
use axum::routing::any;

#[derive(Clone)]
pub struct MapboxStub {
    address: SocketAddr,
    canned: Arc<Mutex<(StatusCode, String)>>,
}

impl MapboxStub {
    pub async fn start() -> anyhow::Result<Self> {
        let canned = Arc::new(Mutex::new((
            StatusCode::OK,
            r#"{"code":"Ok","routes":[]}"#.to_string(),
        )));

        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;

        let served = Arc::clone(&canned);
        let app = Router::new().route(
            "/{*path}",
            any(move || {
                let served = Arc::clone(&served);
                async move {
                    let (status, body) = served.lock().expect("stub lock").clone();
                    Response::builder()
                        .status(status)
                        .header("content-type", "application/json")
                        .body(Body::from(body))
                        .expect("the stub response is well formed")
                }
            }),
        );

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        Ok(Self { address, canned })
    }

    pub fn base_url(&self) -> String {
        format!("http://{}", self.address)
    }

    pub fn answer_with(&self, status: StatusCode, body: impl Into<String>) {
        *self.canned.lock().expect("stub lock") = (status, body.into());
    }
}
