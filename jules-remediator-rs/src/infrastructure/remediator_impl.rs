use crate::domain::models::*;
use crate::domain::security::SecurityValidator;
use crate::domain::services::Remediator;
use crate::infrastructure::git_client::GitClient;
use crate::infrastructure::jules_dispatcher::JulesDispatcher;
use crate::infrastructure::persistence::SurrealPersistence;
use crate::infrastructure::startup_monitor::StartupMonitor;
use anyhow::Result;
use kube::Api;
use std::sync::Arc;
use uuid::Uuid;

pub struct RemediatorImpl {
    dispatcher: Arc<JulesDispatcher>,
    persistence: Arc<SurrealPersistence>,
    git_client: Arc<GitClient>,
    startup_monitor: Arc<StartupMonitor>,
}

impl RemediatorImpl {
    pub async fn new(dispatcher_uri: &str, db_path: &str, git_repo_path: &str) -> Result<Self> {
        let persistence = Arc::new(SurrealPersistence::new(db_path).await?);
        Ok(Self {
            dispatcher: Arc::new(JulesDispatcher::new(dispatcher_uri).await?),
            persistence: persistence.clone(),
            git_client: Arc::new(GitClient::new(git_repo_path.into())),
            startup_monitor: Arc::new(StartupMonitor::new(persistence)),
        })
    }

    pub fn get_startup_monitor(&self) -> Arc<StartupMonitor> {
        self.startup_monitor.clone()
    }
}

#[async_trait::async_trait]
impl Remediator for RemediatorImpl {
    fn classify_error(&self, error: &ClusterError) -> bool {
        // DDD Rule: Structural vs Transient.
        !error.message.contains("transient")
            && (error.error_code == "OOMKilled" || error.error_code == "BackOff")
    }

    async fn propose_fix(&self, error: &ClusterError) -> Result<FixProposal> {
        self.persistence.save_error(error).await?;
        self.dispatcher.get_fix(error).await
    }

    async fn execute_fix(&self, proposal: &FixProposal) -> Result<RemediationOutcome> {
        let tracking_id = proposal.tracking_id;
        println!(
            "[Remediator] Executing fix proposal: {} (Context: {})",
            proposal.proposal_id,
            &tracking_id.to_string()[..8]
        );

        // Security check
        SecurityValidator::validate_proposal(proposal)?;

        let mut logs = String::new();
        let mut success = false;

        if let Some(cmd) = &proposal.remediation_command {
            println!("[Remediator] Running command: {}", cmd);

            let output = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .output()
                .await?;

            success = output.status.success();
            logs = format!(
                "STDOUT:\n{}\nSTDERR:\n{}\n",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );

            // Save step to persistence
            self.persistence
                .save_step(&RemediationStep {
                    tracking_id: proposal.tracking_id,
                    timestamp: chrono::Utc::now(),
                    command: cmd.clone(),
                    success: output.status.success(),
                    exit_code: output.status.code().unwrap_or(-1),
                    stdout: String::from_utf8_lossy(&output.stdout).into(),
                    stderr: String::from_utf8_lossy(&output.stderr).into(),
                })
                .await?;
        } else {
            logs.push_str("No remediation command provided in proposal.");
        }

        let outcome = RemediationOutcome {
            proposal_id: proposal.proposal_id,
            tracking_id: proposal.tracking_id,
            success,
            latency_ms: 0,
            logs,
        };

        self.persistence.save_outcome(&outcome).await?;
        Ok(outcome)
    }

    async fn refine_fix(&self, tracking_id: Uuid, feedback: &str) -> Result<FixProposal> {
        self.dispatcher
            .refine_fix(Uuid::nil(), tracking_id, feedback)
            .await
    }

    async fn verify_resource(&self, resource: &ClusterResource) -> Result<bool> {
        println!(
            "[Remediator] Verifying health of {} in namespace {}",
            resource.name, resource.namespace
        );

        let client = kube::Client::try_default().await?;
        match resource.kind.as_str() {
            "Pod" => {
                let pods: Api<k8s_openapi::api::core::v1::Pod> =
                    Api::namespaced(client, &resource.namespace);
                if let Ok(pod) = pods.get(&resource.name).await {
                    let phase_ok = pod
                        .status
                        .as_ref()
                        .and_then(|status| status.phase.as_deref())
                        .is_some_and(|p| p == "Running" || p == "Succeeded");

                    let ready_ok = pod
                        .status
                        .as_ref()
                        .and_then(|status| status.conditions.as_ref())
                        .map(|conds| {
                            conds
                                .iter()
                                .any(|c| c.type_ == "Ready" && c.status == "True")
                        })
                        .unwrap_or(false);

                    return Ok(phase_ok && ready_ok);
                }
            }
            "Deployment" => {
                let deployments: Api<k8s_openapi::api::apps::v1::Deployment> =
                    Api::namespaced(client, &resource.namespace);
                if let Some(status) = deployments
                    .get(&resource.name)
                    .await
                    .ok()
                    .and_then(|d| d.status)
                {
                    return Ok(status.ready_replicas.unwrap_or(0) > 0);
                }
            }
            _ => {
                println!(
                    "[Remediator] Verification logic for {} not yet implemented",
                    resource.kind
                );
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn create_gitops_pr(&self, proposal: &FixProposal) -> Result<()> {
        println!(
            "[Remediator] Creating GitOps PR for context {}",
            &proposal.tracking_id.to_string()[..8]
        );

        let branch_name = format!("remediation/{}", proposal.tracking_id);
        self.git_client.create_branch(&branch_name)?;

        let log_file = self.git_client.repo_path.join("remediations.log");
        let mut content = std::fs::read_to_string(&log_file).unwrap_or_default();
        content.push_str(&format!(
            "\n--- Context {} ---\n{}\n",
            &proposal.tracking_id.to_string()[..8],
            proposal.code_change
        ));
        std::fs::write(&log_file, content)?;

        self.git_client.commit_all(&format!(
            "Remediation fix for context {}",
            &proposal.tracking_id.to_string()[..8]
        ))?;
        self.git_client.push(&branch_name)?;

        println!(
            "[Remediator] Successfully created and pushed remediation branch: {}",
            branch_name
        );
        Ok(())
    }

    async fn get_startup_state(&self) -> Result<ClusterStartupState> {
        self.startup_monitor.get_current_state().await
    }

    async fn pause_resource(&self, resource: &ClusterResource) -> Result<()> {
        println!(
            "[Remediator] Scaling DOWN resource (SSA): Kind={}, Namespace={}, Name={}",
            resource.kind, resource.namespace, resource.name
        );

        let client = kube::Client::try_default().await?;
        let api_version = if resource.api_version.is_empty() {
            "apps/v1".to_string()
        } else {
            resource.api_version.clone()
        };

        let patch = serde_json::json!({
            "apiVersion": api_version,
            "kind": resource.kind,
            "metadata": {
                "name": resource.name,
                "namespace": resource.namespace,
            },
            "spec": {
                "replicas": 0
            }
        });

        let params = kube::api::PatchParams::apply("jules-remediator").force();

        match resource.kind.as_str() {
            "Deployment" => {
                let api: Api<k8s_openapi::api::apps::v1::Deployment> =
                    Api::namespaced(client, &resource.namespace);
                api.patch(&resource.name, &params, &kube::api::Patch::Apply(patch))
                    .await?;
            }
            "StatefulSet" => {
                let api: Api<k8s_openapi::api::apps::v1::StatefulSet> =
                    Api::namespaced(client, &resource.namespace);
                api.patch(&resource.name, &params, &kube::api::Patch::Apply(patch))
                    .await?;
            }
            _ => {
                println!("[Remediator] Warning: Scaling NOT supported for kind {}", resource.kind);
                anyhow::bail!("Scaling for kind {} not implemented", resource.kind)
            },
        }

        Ok(())
    }

    async fn resume_resource(&self, resource: &ClusterResource) -> Result<()> {
        println!(
            "[Remediator] Scaling UP resource (SSA): Kind={}, Namespace={}, Name={}",
            resource.kind, resource.namespace, resource.name
        );

        let client = kube::Client::try_default().await?;
        let api_version = if resource.api_version.is_empty() {
            "apps/v1".to_string()
        } else {
            resource.api_version.clone()
        };

        let patch = serde_json::json!({
            "apiVersion": api_version,
            "kind": resource.kind,
            "metadata": {
                "name": resource.name,
                "namespace": resource.namespace,
            },
            "spec": {
                "replicas": 1
            }
        });

        let params = kube::api::PatchParams::apply("jules-remediator").force();

        match resource.kind.as_str() {
            "Deployment" => {
                let api: Api<k8s_openapi::api::apps::v1::Deployment> =
                    Api::namespaced(client, &resource.namespace);
                api.patch(&resource.name, &params, &kube::api::Patch::Apply(patch))
                    .await?;
            }
            "StatefulSet" => {
                let api: Api<k8s_openapi::api::apps::v1::StatefulSet> =
                    Api::namespaced(client, &resource.namespace);
                api.patch(&resource.name, &params, &kube::api::Patch::Apply(patch))
                    .await?;
            }
            _ => {
                println!("[Remediator] Warning: Scaling NOT supported for kind {}", resource.kind);
                anyhow::bail!("Scaling for kind {} not implemented", resource.kind)
            },
        }

        Ok(())
    }

    async fn check_startup_dependency(&self, resource: &ClusterResource) -> Result<Option<String>> {
        self.startup_monitor
            .is_waiting_for_dependency(resource)
            .await
    }

    async fn list_resources(&self, namespace: &str) -> Result<Vec<ClusterResource>> {
        let client = kube::Client::try_default().await?;
        let mut resources = Vec::new();

        // 1. Deployments
        let deployments: Api<k8s_openapi::api::apps::v1::Deployment> =
            Api::namespaced(client.clone(), namespace);
        let d_list = deployments.list(&kube::api::ListParams::default()).await?;
        for d in d_list.items {
            if let Some(name) = d.metadata.name {
                resources.push(ClusterResource {
                    kind: "Deployment".to_string(),
                    name,
                    namespace: namespace.to_string(),
                    api_version: "apps/v1".to_string(),
                });
            }
        }

        // 2. StatefulSets
        let statefulsets: Api<k8s_openapi::api::apps::v1::StatefulSet> =
            Api::namespaced(client.clone(), namespace);
        let s_list = statefulsets.list(&kube::api::ListParams::default()).await?;
        for s in s_list.items {
            if let Some(name) = s.metadata.name {
                resources.push(ClusterResource {
                    kind: "StatefulSet".to_string(),
                    name,
                    namespace: namespace.to_string(),
                    api_version: "apps/v1".to_string(),
                });
            }
        }

        Ok(resources)
    }

    async fn get_tier_resources(&self, tier: DependencyTier) -> Result<Vec<ClusterResource>> {
        self.startup_monitor.get_resources_for_tier(tier).await
    }
}
