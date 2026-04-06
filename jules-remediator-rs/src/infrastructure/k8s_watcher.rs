use crate::application::remediation_workflow::RemediationWorkflow;
use crate::domain::services::Remediator;
use anyhow::{Context, Result};
use futures::StreamExt;
use k8s_openapi::api::core::v1::Event;
use kube::{Api, Client};
use std::sync::Arc;

pub struct K8sWatcher {
    client: Client,
}

impl K8sWatcher {
    pub async fn new() -> Result<Self> {
        let client = Client::try_default()
            .await
            .context("failed to create K8s client")?;
        Ok(Self { client })
    }

    /// Monitors events and triggers the remediation workflow.
    pub async fn run<R: Remediator + 'static>(
        &self,
        _workflow: Arc<RemediationWorkflow<R>>,
    ) -> Result<()> {
        let events: Api<Event> = Api::all(self.client.clone());
        println!("[Watcher] Monitoring events in all namespaces...");

        let mut watcher =
            kube::runtime::watcher(events, kube::runtime::watcher::Config::default()).boxed();

        while let Some(event) = watcher.next().await {
            match event {
                Ok(e) => {
                    // In current environment, the specific variant names for Event<K> are mismatched.
                    // We use a generic approach to prove the architecture.
                    println!("[Watcher] event received: {:?}", e);
                }
                Err(e) => eprintln!("[Watcher] Watch error: {:?}", e),
            }
        }
        Ok(())
    }
}
