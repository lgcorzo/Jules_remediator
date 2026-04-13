use crate::domain::models::*;
use crate::infrastructure::persistence::SurrealPersistence;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::sync::Arc;

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
        println!(
            "[StartupMonitor] Event: {} {} is {}",
            event.resource.kind, event.resource.name, event.status
        );
        self.persistence.save_startup_event(&event).await
    }

    pub async fn get_current_state(&self) -> Result<ClusterStartupState> {
        let timeline = self.persistence.get_startup_timeline().await?;
        let event_count = timeline.len();

        let phase = if event_count < 5 {
            StartupPhase::Initial
        } else if Utc::now()
            .signed_duration_since(self.start_time)
            .num_minutes()
            < 10
        {
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
    pub async fn is_waiting_for_dependency(
        &self,
        resource: &ClusterResource,
    ) -> Result<Option<String>> {
        // Simple heuristic: If we are in startup and another service in the same namespace
        // that is often a dependency (e.g. redis, postgres, mariadb) is NOT yet ready.
        let timeline = self.persistence.get_startup_timeline().await?;

        let common_deps = [
            "mysql", "mariadb", "postgres", "redis", "mongodb", "rabbitmq",
        ];

        for dep_name in common_deps {
            // Check if there's a resource with this name in the same namespace
            // and see if it has a "Ready" event in the timeline.
            let is_ready = timeline.iter().any(|e| {
                e.resource.namespace == resource.namespace
                    && e.resource.name.contains(dep_name)
                    && e.status == "Ready"
            });

            if !is_ready {
                // Check if such a resource actually EXISTS (or was attempted to start)
                let exists = timeline.iter().any(|e| {
                    e.resource.namespace == resource.namespace && e.resource.name.contains(dep_name)
                });

                if exists {
                    return Ok(Some(dep_name.to_string()));
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_startup_phase_transitions() {
        let persistence = Arc::new(SurrealPersistence::new("").await.unwrap());
        let monitor = StartupMonitor::new(persistence.clone());

        let state = monitor.get_current_state().await.unwrap();
        assert!(matches!(state.phase, StartupPhase::Initial));

        // Add events
        for i in 0..6 {
            monitor
                .record_event(StartupEvent {
                    timestamp: Utc::now(),
                    resource: ClusterResource {
                        kind: "Pod".into(),
                        name: format!("pod-{}", i),
                        namespace: "default".into(),
                        api_version: "v1".into(),
                    },
                    status: "Ready".into(),
                })
                .await
                .unwrap();
        }

        let state = monitor.get_current_state().await.unwrap();
        assert!(matches!(state.phase, StartupPhase::InProcess));
    }

    #[tokio::test]
    async fn test_dependency_inference() {
        let persistence = Arc::new(SurrealPersistence::new("").await.unwrap());
        let monitor = StartupMonitor::new(persistence.clone());

        let app_resource = ClusterResource {
            kind: "Pod".into(),
            name: "web-app".into(),
            namespace: "default".into(),
            api_version: "v1".into(),
        };

        // No events yet
        let dep = monitor
            .is_waiting_for_dependency(&app_resource)
            .await
            .unwrap();
        assert!(dep.is_none());

        // Record a "Started" (but not yet Ready) mysql event
        monitor
            .record_event(StartupEvent {
                timestamp: Utc::now(),
                resource: ClusterResource {
                    kind: "Pod".into(),
                    name: "mysql-0".into(),
                    namespace: "default".into(),
                    api_version: "v1".into(),
                },
                status: "Started".into(),
            })
            .await
            .unwrap();

        let dep = monitor
            .is_waiting_for_dependency(&app_resource)
            .await
            .unwrap();
        assert_eq!(dep, Some("mysql".to_string()));

        // Make it Ready
        monitor
            .record_event(StartupEvent {
                timestamp: Utc::now(),
                resource: ClusterResource {
                    kind: "Pod".into(),
                    name: "mysql-0".into(),
                    namespace: "default".into(),
                    api_version: "v1".into(),
                },
                status: "Ready".into(),
            })
            .await
            .unwrap();

        let dep = monitor
            .is_waiting_for_dependency(&app_resource)
            .await
            .unwrap();
        assert!(dep.is_none());
    }
}
