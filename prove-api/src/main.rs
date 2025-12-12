use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};

#[derive(Clone)]
struct AppState {
    start_time: std::time::Instant,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    uptime_secs: u64,
}

#[derive(Deserialize)]
struct GenerateProofRequest {
    sender_domain: String,
    sender_raw: String,
    receiver_domain: String,
    receiver_raw: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

async fn ping(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        uptime_secs: state.start_time.elapsed().as_secs(),
    })
}

async fn generate_proof(
    Json(payload): Json<GenerateProofRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Received proof generation request. Sender domain: {}, Receiver domain: {}",
        payload.sender_domain, payload.receiver_domain
    );

    let receipt = host::verify_email_pair(
        &payload.sender_domain,
        &payload.sender_raw,
        &payload.receiver_domain,
        &payload.receiver_raw,
    )
    .await
    .map_err(|e| {
        error!("Proof generation failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let receipt_bytes = bincode::serialize(&receipt).map_err(|e| {
        error!("Failed to serialize receipt: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to serialize receipt: {}", e),
            }),
        )
    })?;

    info!(
        "Proof generated successfully. Receipt size: {} bytes",
        receipt_bytes.len()
    );

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
        receipt_bytes,
    ))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("prove_api=info".parse().unwrap())
                .add_directive("host=info".parse().unwrap()),
        )
        .init();

    let state = Arc::new(AppState {
        start_time: std::time::Instant::now(),
    });

    let app = Router::new()
        .route("/ping", get(ping))
        .route("/generate", post(generate_proof))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("PORT must be a valid u16");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting prove-api server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
