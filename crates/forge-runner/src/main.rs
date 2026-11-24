mod agent;
mod heartbeat;
mod workspace;

use forge_observability::init_tracing;
use tracing::info;

#[tokio::main]
async fn main() {
    init_tracing();
    info!("starting forge-runner");
    agent::run().await;
}