use axum::{
    routing::get,
    http::StatusCode,
    Json, Router,
};
use serde::Serialize;
use std::net::SocketAddr;

#[derive(Serialize)]
struct StatusResponse {
    name: String,
    version: String,
    status: String,
    description: String,
}

async fn root() -> &'static str {
    "DeepEdge API - Don't Bet Blind. See the Math."
}

async fn health() -> Json<StatusResponse> {
    Json(StatusResponse {
        name: "DeepEdge".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        status: "OK".to_string(),
        description: "Data-driven prediction market platform on DeepBook Predict".to_string(),
    })
}

async fn fallback() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .fallback(fallback);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("DeepEdge API starting on http://{}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
