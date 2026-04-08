use crate::application::RemediationWorkflow;
use crate::domain::models::*;
use crate::domain::services::Remediator;
use anyhow::{Context, Result};
use futures::StreamExt;
use k8s_openapi::api::core::v1::Event;
use kube::{Api, Client};
use std::sync::Arc;
use uuid::Uuid;

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

    /// Internal constructor for testing or custom configuration.
    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    /// Monitors events and triggers the remediation workflow.
    pub async fn run<R: Remediator + 'static>(
        &self,
        workflow: Arc<RemediationWorkflow<R>>,
    ) -> Result<()> {
        let events: Api<Event> = Api::all(self.client.clone());
        println!("[Watcher] Monitoring events in all namespaces...");

        let mut watcher =
            kube::runtime::watcher(events, kube::runtime::watcher::Config::default()).boxed();

        while let Some(event) = watcher.next().await {
            match event {
                Ok(kube::runtime::watcher::Event::Apply(e))
                | Ok(kube::runtime::watcher::Event::InitApply(e)) => {
                    Self::handle_event_logic(e, workflow.clone()).await?;
                }
                Ok(kube::runtime::watcher::Event::Init) => {
                    println!("[Watcher] Initializing event sync...");
                }
                Ok(kube::runtime::watcher::Event::InitDone) => {
                    println!("[Watcher] Initial event sync complete.");
                }
                Ok(kube::runtime::watcher::Event::Delete(_)) => {}
                Err(e) => eprintln!("[Watcher] Watch error: {:?}", e),
            }
        }
        Ok(())
    }

    pub async fn handle_event_logic<R: Remediator + 'static>(
        e: Event,
        workflow: Arc<RemediationWorkflow<R>>,
    ) -> Result<()> {
        // Filter for errors (e.g., Warning events with OOMKilled or BackOff)
        if e.type_ == Some("Warning".into()) {
            let reason = e.reason.clone().unwrap_or_default();
            if reason == "OOMKilled" || reason == "BackOff" {
                println!(
                    "[Watcher] Detected target error: {} in {}",
                    reason,
                    e.metadata.name.clone().unwrap_or_default()
                );

                let cluster_error = ClusterError {
                    id: Uuid::new_v4(),
                    timestamp: chrono::Utc::now(),
                    severity: Severity::High,
                    error_type: ErrorType::Unknown,
                    resource: ClusterResource {
                        kind: e.involved_object.kind.clone().unwrap_or_default(),
                        name: e.involved_object.name.clone().unwrap_or_default(),
                        namespace: e.involved_object.namespace.clone().unwrap_or_default(),
                        api_version: e.involved_object.api_version.clone().unwrap_or_default(),
                    },
                    message: e.message.clone().unwrap_or_default(),
                    error_code: reason,
                    raw_event: serde_json::to_value(&e).unwrap_or(serde_json::Value::Null),
                };

                let _ = workflow.handle_error(cluster_error).await;
            }
        }
        Ok(())
    }
}
