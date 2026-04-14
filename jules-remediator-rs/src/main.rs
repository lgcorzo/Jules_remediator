use anyhow::Result;
use jules_remediator_rs::application::RemediationWorkflow;
use jules_remediator_rs::infrastructure::{K8sWatcher, RemediatorImpl};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting Jules Remediator (Rust Edition)...");

    // Load configuration (in a real scenario from an env or TOML file)
    let dispatcher_uri = std::env::var("JULES_DISPATCHER_URI")
        .unwrap_or_else(|_| "http://jules-cloud-vm.internal:8080/mcp".into());
    let db_path = "surreal.db";
    let git_repo_path = std::env::var("GITOPS_REPO_PATH")
        .unwrap_or_else(|_| "/mnt/F024B17C24B145FE/Repos/gitops_internal_lgcorzo".into());

    // Layer 1: Infrastructure (Adapters)
    let remediator = Arc::new(RemediatorImpl::new(&dispatcher_uri, db_path, &git_repo_path).await?);
    let startup_monitor = remediator.get_startup_monitor();

    // Layer 2: Application (Use Case)
    let workflow = Arc::new(RemediationWorkflow::new(remediator.clone()));
    let startup_master = jules_remediator_rs::application::StartupMaster::new(remediator.clone());

    // Layer 2.5: Background Master Loop
    tokio::spawn(async move {
        if let Err(e) = startup_master.run().await {
            eprintln!("[StartupMaster] Critical Loop Failure: {:?}", e);
        }
    });

    // Layer 3: Watcher Loop (Primary Adapter)
    let watcher = K8sWatcher::new(Some(startup_monitor)).await?;
    watcher.run(workflow).await?;

    Ok(())
}
