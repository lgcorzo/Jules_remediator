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
    startup_monitor: Option<Arc<StartupMonitor>>,
}

impl K8sWatcher {
    pub async fn new(startup_monitor: Option<Arc<StartupMonitor>>) -> Result<Self> {
        let client = Client::try_default()
            .await
            .context("failed to create K8s client")?;
        Ok(Self { client, startup_monitor })
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
                    self.process_event(e, workflow.clone()).await?;
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

    async fn process_event<R: Remediator + 'static>(
        &self,
        e: Event,
        workflow: Arc<RemediationWorkflow<R>>,
    ) -> Result<()> {
        let reason = e.reason.clone().unwrap_or_default();
        let message = e.message.clone().unwrap_or_default();
        let type_ = e.type_.clone().unwrap_or_default();

        let resource = ClusterResource {
            kind: e.involved_object.kind.clone().unwrap_or_default(),
            name: e.involved_object.name.clone().unwrap_or_default(),
            namespace: e.involved_object.namespace.clone().unwrap_or_default(),
            api_version: e.involved_object.api_version.clone().unwrap_or_default(),
        };

        // Track Startup Events (Normal)
        if type_ == "Normal" {
            if let Some(ref monitor) = self.startup_monitor {
                if reason == "Started" || reason == "Ready" {
                    monitor.record_event(StartupEvent {
                        timestamp: chrono::Utc::now(),
                        resource: resource.clone(),
                        status: reason.clone(),
                    }).await?;
                }
            }
        }

        // Filter for errors (Warning)
        if type_ == "Warning" {
            if reason == "OOMKilled" || reason == "BackOff" {
                println!(
                    "[Watcher] Detected target error: {} in {}",
                    reason,
                    resource.name
                );

                let cluster_error = ClusterError {
                    id: Uuid::new_v4(),
                    timestamp: chrono::Utc::now(),
                    severity: Severity::High,
                    error_type: ErrorType::Structural,
                    resource,
                    message,
                    error_code: reason,
                    raw_event: serde_json::to_value(&e).unwrap_or(serde_json::Value::Null),
                };

                let _ = workflow.handle_error(cluster_error).await;
            }
        }
        Ok(())
    }
}
