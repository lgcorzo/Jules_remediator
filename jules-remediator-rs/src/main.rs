use anyhow::Result;
use jules_remediator_rs::infrastructure::{K8sWatcher, RemediatorImpl};
use jules_remediator_rs::application::RemediationWorkflow;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting Jules Remediator (Rust Edition)...");

    // Load configuration (in a real scenario from an env or TOML file)
    let dispatcher_uri = std::env::var("JULES_DISPATCHER_URI").unwrap_or_else(|_| "http://jules-cloud-vm.internal:8080/mcp".into());
    let mlflow_uri = std::env::var("MLFLOW_TRACKING_URI").unwrap_or_else(|_| "http://mlflow.ml-system.svc.cluster.local:5000".into());
    let db_path = "surreal.db";

    // Layer 1: Infrastructure (Adapters)
    let remediator = Arc::new(RemediatorImpl::new(
        &dispatcher_uri,
        &mlflow_uri,
        db_path,
    ).await?);

    // Layer 2: Application (Use Case)
    let workflow = Arc::new(RemediationWorkflow::new(remediator.clone()));

    // Layer 3: Watcher Loop (Primary Adapter)
    let watcher = K8sWatcher::new().await?;
    watcher.run(workflow).await?;

    Ok(())
}
