mod api;
mod auth;
mod grpc;
mod middleware;
mod state;

use axum::middleware::from_fn;
use forge_observability::init_tracing;
use state::AppState;
use std::net::SocketAddr;
use std::time::Duration;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() {
    init_tracing();
    let state = AppState::bootstrap()
        .await
        .expect("failed to bootstrap forge-server");
    if let Err(error) =
        forge_storage::repos::executions::mark_expired_jobs_unknown(state.storage.pool()).await
    {
        tracing::error!(%error, "failed to recover expired job leases");
    }
    if let Err(error) =
        forge_storage::repos::executions::reconcile_runner_job_counts(state.storage.pool()).await
    {
        tracing::error!(%error, "failed to reconcile runner job counts");
    }
    let recovery_storage = state.storage.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Err(error) =
                forge_storage::repos::executions::mark_expired_jobs_unknown(recovery_storage.pool())
                    .await
            {
                tracing::error!(%error, "job lease recovery failed");
            }
        }
    });

    let grpc_state = state.clone();
    let grpc_addr: SocketAddr = ([0, 0, 0, 0], grpc_port()).into();
    tokio::spawn(async move {
        if let Err(error) = crate::grpc::serve(grpc_state, grpc_addr).await {
            tracing::error!(%error, %grpc_addr, "runner gRPC server stopped");
        }
    });

    let app = api::router(state)
        .layer(TraceLayer::new_for_http())
        .layer(from_fn(crate::middleware::security_headers));

    let addr = SocketAddr::from(([0, 0, 0, 0], server_port()));
    info!(%addr, "starting forge-server");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind forge-server listener");
    axum::serve(listener, app)
        .await
        .expect("forge-server stopped unexpectedly");
    forge_observability::shutdown_otel();
}

fn server_port() -> u16 {
    std::env::var("FORGE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080)
}

fn grpc_port() -> u16 {
    std::env::var("FORGE_GRPC_PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(8081)
}
