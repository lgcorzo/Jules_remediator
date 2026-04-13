use crate::domain::models::*;
use crate::infrastructure::persistence::SurrealPersistence;
use anyhow::Result;
use std::sync::Arc;
use chrono::{DateTime, Utc};

pub struct StartupMonitor {
    persistence: Arc<SurrealPersistence>,
    start_time: DateTime<Utc>,
}

impl StartupMonitor {
    pub fn new(persistence: Arc<SurrealPersistence>) -> Self {
        Self {
            persistence,
            start_time: Utc::now(),
        }
    }

    pub async fn record_event(&self, event: StartupEvent) -> Result<()> {
        println!("[StartupMonitor] Event: {} {} is {}", event.resource.kind, event.resource.name, event.status);
        self.persistence.save_startup_event(&event).await
    }

    pub async fn get_current_state(&self) -> Result<ClusterStartupState> {
        let timeline = self.persistence.get_startup_timeline().await?;
        let event_count = timeline.len();
        
        let phase = if event_count < 5 {
            StartupPhase::Initial
        } else if Utc::now().signed_duration_since(self.start_time).num_minutes() < 10 {
            StartupPhase::InProcess
        } else {
            StartupPhase::Stabilized
        };

        Ok(ClusterStartupState {
            phase,
            event_count,
            start_time: self.start_time,
        })
    }

    /// Checks if a resource likely depends on another that isn't yet ready.
    pub async fn is_waiting_for_dependency(&self, resource: &ClusterResource) -> Result<Option<String>> {
        // Simple heuristic: If we are in startup and another service in the same namespace 
        // that is often a dependency (e.g. redis, postgres, mariadb) is NOT yet ready.
        let timeline = self.persistence.get_startup_timeline().await?;
        
        let common_deps = ["mysql", "mariadb", "postgres", "redis", "mongodb", "rabbitmq"];
        
        for dep_name in common_deps {
            // Check if there's a resource with this name in the same namespace
            // and see if it has a "Ready" event in the timeline.
            let is_ready = timeline.iter().any(|e| 
                e.resource.namespace == resource.namespace && 
                e.resource.name.contains(dep_name) && 
                e.status == "Ready"
            );

            if !is_ready {
                // Check if such a resource actually EXISTS (or was attempted to start)
                let exists = timeline.iter().any(|e| 
                    e.resource.namespace == resource.namespace && 
                    e.resource.name.contains(dep_name)
                );

                if exists {
                    return Ok(Some(dep_name.to_string()));
                }
            }
        }

        Ok(None)
    }
}
