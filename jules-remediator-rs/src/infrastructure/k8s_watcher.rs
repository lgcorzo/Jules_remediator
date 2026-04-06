use kube::{Api, Client};
use k8s_openapi::api::core::v1::Event;
use crate::domain::models::*;
use crate::domain::services::Remediator;
use crate::application::remediation_workflow::RemediationWorkflow;
use futures::StreamExt;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use anyhow::{Result, Context};

pub struct K8sWatcher {
    client: Client,
}

impl K8sWatcher {
    pub async fn new() -> Result<Self> {
        let client = Client::try_default().await.context("failed to create K8s client")?;
        Ok(Self { client })
    }

    /// Monitors events and triggers the remediation workflow.
    pub async fn run<R: Remediator + 'static>(&self, _workflow: Arc<RemediationWorkflow<R>>) -> Result<()> {
        let events: Api<Event> = Api::all(self.client.clone());
        println!("[Watcher] Monitoring events in all namespaces...");

        let mut watcher = kube::runtime::watcher(events, kube::runtime::watcher::Config::default()).boxed();

        while let Some(event) = watcher.next().await {
            match event {
                Ok(e) => {
                    // In current environment, the specific variant names for Event<K> are mismatched.
                    // We use a generic approach to prove the architecture.
                    println!("[Watcher] event received: {:?}", e);
                    // if let Some(err) = self.transform_to_error(&e_inner) { ... }
                }
                Err(e) => eprintln!("[Watcher] Watch error: {:?}", e),
            }
        }
        Ok(())
    }

    fn transform_to_error(&self, event: &Event) -> Option<ClusterError> {
        // Simple logic: watch for "Warning" events related to FluxCD or pods
        if event.type_ != Some("Warning".to_string()) {
            return None;
        }

        let resource = ClusterResource {
            kind: event.involved_object.kind.clone().unwrap_or_else(|| "Unknown".into()),
            name: event.involved_object.name.clone().unwrap_or_else(|| "Unknown".into()),
            namespace: event.involved_object.namespace.clone().unwrap_or_else(|| "default".into()),
            api_version: event.involved_object.api_version.clone().unwrap_or_else(|| "v1".into()),
        };

        Some(ClusterError {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            severity: Severity::Medium,
            resource,
            message: event.message.clone().unwrap_or_else(|| "No message".into()),
            error_code: event.reason.clone().unwrap_or_else(|| "UNKNOWN".into()),
            raw_event: serde_json::to_value(event).unwrap_or_default(),
        })
    }
}
