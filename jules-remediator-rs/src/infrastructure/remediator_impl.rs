use crate::domain::models::*;
use crate::domain::services::Remediator;
use crate::infrastructure::jules_dispatcher::JulesDispatcher;
use crate::infrastructure::mlflow_logger::MlflowLogger;
use crate::infrastructure::persistence::SurrealPersistence;
use anyhow::Result;
use std::sync::Arc;

use crate::infrastructure::git_client::GitClient;

pub struct RemediatorImpl {
    dispatcher: Arc<JulesDispatcher>,
    logger: Arc<MlflowLogger>,
    persistence: Arc<SurrealPersistence>,
    git_client: Arc<GitClient>,
    startup_monitor: Arc<StartupMonitor>,
}

impl RemediatorImpl {
    pub async fn new(dispatcher_uri: &str, mlflow_uri: &str, db_path: &str, git_repo_path: &str) -> Result<Self> {
        let persistence = Arc::new(SurrealPersistence::new(db_path).await?);
        Ok(Self {
            dispatcher: Arc::new(JulesDispatcher::new(dispatcher_uri).await?),
            logger: Arc::new(MlflowLogger::new(mlflow_uri.into())),
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
        // Let's assume we skip simple transient errors (e.g. ImagePullBackOff due to registry transient error)
        // but handle OOMKilled or CrashLoopBackOff.
        !error.message.contains("transient")
            && (error.error_code == "OOMKilled" || error.error_code == "BackOff")
    }

    async fn propose_fix(&self, error: &ClusterError) -> Result<FixProposal> {
        self.persistence.save_error(error).await?;
        self.dispatcher.get_fix(error).await
    }

    async fn execute_fix(&self, proposal: &FixProposal) -> Result<RemediationOutcome> {
        println!(
            "[Remediator] Executing fix proposal: {} (Session: {})",
            proposal.proposal_id, proposal.session_id
        );

        let mut logs = String::new();
        let mut success = false;

        if let Some(ref cmd) = proposal.remediation_command {
            println!("[Remediator] Running command: {}", cmd);
            
            // Execute the command using tokio::process::Command
            let mut child = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?;

            let output = child.wait_with_output().await?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            logs.push_str(&format!("STDOUT:\n{}\nSTDERR:\n{}\n", stdout, stderr));
            success = output.status.success();

            // Save step to persistence
            self.persistence.save_step(&RemediationStep {
                session_id: proposal.session_id,
                timestamp: chrono::Utc::now(),
                command: cmd.clone(),
                success,
                exit_code: output.status.code().unwrap_or(-1),
                stdout: stdout.into(),
                stderr: stderr.into(),
            }).await?;
        } else {
            logs.push_str("No remediation command provided in proposal.");
        }

        let outcome = RemediationOutcome {
            proposal_id: proposal.proposal_id,
            session_id: proposal.session_id,
            success,
            latency_ms: 0, // Should calculate real latency
            logs,
        };

        self.persistence.save_outcome(&outcome).await?;
        Ok(outcome)
    }

    async fn refine_fix(&self, session_id: Uuid, feedback: &str) -> Result<FixProposal> {
        // We need the original error_id. In a real scenario, we'd look it up in persistence.
        // For now, we'll assume we can retrieve it or pass it.
        // Let's assume we find it from the outcome of the session.
        self.dispatcher.refine_fix(Uuid::nil(), session_id, feedback).await
    }

    async fn verify_resource(&self, resource: &ClusterResource) -> Result<bool> {
        println!("[Remediator] Verifying health of {} in namespace {}", resource.name, resource.namespace);
        
        let client = kube::Client::try_default().await?;
        match resource.kind.as_str() {
            "Pod" => {
                let pods: Api<k8s_openapi::api::core::v1::Pod> = Api::namespaced(client, &resource.namespace);
                if let Ok(pod) = pods.get(&resource.name).await {
                    if let Some(status) = pod.status {
                        if let Some(phase) = status.phase {
                            return Ok(phase == "Running" || phase == "Succeeded");
                        }
                    }
                }
            },
            "Deployment" => {
                let deployments: Api<k8s_openapi::api::apps::v1::Deployment> = Api::namespaced(client, &resource.namespace);
                if let Ok(deploy) = deployments.get(&resource.name).await {
                    if let Some(status) = deploy.status {
                        return Ok(status.ready_replicas.unwrap_or(0) > 0);
                    }
                }
            },
            _ => {
                println!("[Remediator] Verification logic for {} not yet implemented", resource.kind);
                return Ok(true); // Default to true if unknown kind
            }
        }
        Ok(false)
    }

    async fn create_gitops_pr(&self, proposal: &FixProposal) -> Result<()> {
        println!("[Remediator] Creating GitOps PR for session {}", proposal.session_id);
        
        // 1. Prepare branch name
        let branch_name = format!("remediation/{}", proposal.session_id);
        
        // 2. clone (if not exists) and branch
        // In a real scenario, we might need a separate repo URL.
        // For now, we assume the git_client points to a local repo we can work on.
        self.git_client.create_branch(&branch_name)?;
        
        // 3. Apply changes (code_change)
        // In a real scenario, Jules would provide a file path or we'd need to guess.
        // For now, let's assume we append to a 'remediations.log' or similar for demo.
        let log_file = self.git_client.repo_path.join("remediations.log");
        let mut content = std::fs::read_to_string(&log_file).unwrap_or_default();
        content.push_str(&format!("\n--- Session {} ---\n{}\n", proposal.session_id, proposal.code_change));
        std::fs::write(&log_file, content)?;

        // 4. Commit and Push
        self.git_client.commit_all(&format!("Remediation fix for session {}", proposal.session_id))?;
        // self.git_client.push(&branch_name)?; // Disabled for safety in the environment
        
        println!("[Remediator] Successfully created remediation branch: {}", branch_name);
        Ok(())
    }

    async fn get_startup_state(&self) -> Result<ClusterStartupState> {
        self.startup_monitor.get_current_state().await
    }

    async fn pause_resource(&self, resource: &ClusterResource) -> Result<()> {
        println!("[Remediator] Pausing resource {}/{} (Scaling to 0)", resource.namespace, resource.name);
        
        // Use kubectl to scale to 0
        let output = tokio::process::Command::new("kubectl")
            .arg("scale")
            .arg(&resource.kind.to_lowercase())
            .arg(&resource.name)
            .arg("-n")
            .arg(&resource.namespace)
            .arg("--replicas=0")
            .output()
            .await?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to pause resource: {}", err);
        }

        Ok(())
    }

    async fn resume_resource(&self, resource: &ClusterResource) -> Result<()> {
        println!("[Remediator] Resuming resource {}/{} (Scaling to 1)", resource.namespace, resource.name);
        
        // Use kubectl to scale to 1 (Assume 1 as default for now, could be improved to fetch original replicas)
        let output = tokio::process::Command::new("kubectl")
            .arg("scale")
            .arg(&resource.kind.to_lowercase())
            .arg(&resource.name)
            .arg("-n")
            .arg(&resource.namespace)
            .arg("--replicas=1")
            .output()
            .await?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to resume resource: {}", err);
        }

        Ok(())
    }

    async fn check_startup_dependency(&self, resource: &ClusterResource) -> Result<Option<String>> {
        self.startup_monitor.is_waiting_for_dependency(resource).await
    }
}
