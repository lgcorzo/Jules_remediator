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

        let client = match kube::Client::try_default().await {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!("[StartupMonitor] Warning: Could not create K8s client (falling back to heuristic): {:?}", e);
                None
            }
        };

        for dep_name in common_deps {
            let mut live_found = false;
            let mut live_ready = false;

            // 1. Try Live Check
            if let Some(c) = &client {
                let pods: kube::Api<k8s_openapi::api::core::v1::Pod> =
                    kube::Api::namespaced(c.clone(), &resource.namespace);
                
                if let Ok(pod_list) = pods.list(&kube::api::ListParams::default()).await {
                    for pod in pod_list {
                        let name = pod.metadata.name.unwrap_or_default();
                        if name.contains(dep_name) {
                            live_found = true;
                            live_ready = pod.status.and_then(|status| status.conditions).map(|conds| {
                                conds.iter().any(|c| c.type_ == "Ready" && c.status == "True")
                            }).unwrap_or(false);
                            break;
                        }
                    }
                }
            }

            if live_found {
                if live_ready {
                    continue; // Dependency is healthy
                } else {
                    println!("[StartupMonitor] Live check: {} is NOT ready.", dep_name);
                    return Ok(Some(dep_name.to_string())); // Block
                }
            }

            // 2. Fallback to Timeline Heuristic
            let timeline_ready = timeline.iter().any(|e| {
                e.resource.namespace == resource.namespace
                    && e.resource.name.contains(dep_name)
                    && e.status == "Ready"
            });

            if timeline_ready {
                continue;
            }

            let timeline_exists = timeline.iter().any(|e| {
                e.resource.namespace == resource.namespace && e.resource.name.contains(dep_name)
            });

            if timeline_exists {
                return Ok(Some(dep_name.to_string()));
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
