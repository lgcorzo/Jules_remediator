use anyhow::Result;
use jules_remediator_rs::application::RemediationWorkflow;
use jules_remediator_rs::domain::ports::Tracker;
use jules_remediator_rs::infrastructure::{
    K8sWatcher, MlflowLogger, Orchestrator, OrchestratorImpl, RemediatorImpl,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting Jules Remediator (Rust Edition)...");

    // Load configuration
    let dispatcher_uri = std::env::var("JULES_DISPATCHER_URI")
        .unwrap_or_else(|_| "http://jules-cloud-vm.internal:8080/mcp".into());
    let mlflow_uri = std::env::var("MLFLOW_TRACKING_URI")
        .unwrap_or_else(|_| "http://mlflow.ml-system.svc.cluster.local:5000".into());
    let db_path = "surreal.db";

    // Layer 1: Infrastructure (Adapters)
    let orchestrator: Arc<dyn Orchestrator> = Arc::new(OrchestratorImpl::new(&dispatcher_uri));
    let tracker: Arc<dyn Tracker> = Arc::new(MlflowLogger::new(mlflow_uri));

    let remediator =
        Arc::new(RemediatorImpl::new_with_dependencies(db_path, orchestrator, tracker).await?);

    // Layer 2: Application (Use Case)
    let workflow = Arc::new(RemediationWorkflow::new(remediator.clone()));

    // Layer 3: Watcher Loop (Primary Adapter)
    let watcher = K8sWatcher::new().await?;
    watcher.run(workflow).await?;

    Ok(())
}
