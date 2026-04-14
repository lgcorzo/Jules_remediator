use crate::domain::models::*;
use crate::domain::security::SecurityValidator;
use crate::domain::services::Remediator;
use anyhow::Result;
use std::sync::Arc;

pub struct RemediationWorkflow<R: Remediator> {
    remediator: Arc<R>,
}

impl<R: Remediator> RemediationWorkflow<R> {
    pub fn new(remediator: Arc<R>) -> Self {
        Self { remediator }
    }

    /// Handles a cluster error event through the full remediation lifecycle.
    pub async fn handle_error(&self, error: ClusterError) -> Result<Option<RemediationOutcome>> {
        println!("[Workflow] Processing error: {}", error.id);

        if !self.remediator.classify_error(&error) {
            return Ok(None);
        }

        // --- Startup Orchestration ---
        let startup_state = self.remediator.get_startup_state().await?;

        if startup_state.boot_storm_detected
            || matches!(startup_state.phase, StartupPhase::Controlled)
        {
            println!(
                "[Workflow] Controlled Startup active. Current phase: {:?}",
                startup_state.phase
            );

            // Tier 0: Bootstrap Anchor Check (flux-system/source-controller Deployment)
            let bootstrap_resource = ClusterResource {
                kind: "Deployment".into(),
                name: "source-controller".into(),
                namespace: "flux-system".into(),
                api_version: "apps/v1".into(),
            };

            if !self
                .remediator
                .verify_resource(&bootstrap_resource)
                .await
                .unwrap_or(false)
            {
                println!("[Workflow] Tier 0 (Bootstrap) not ready. Ensuring source-controller...");
                // In a real scenario, this would trigger a scale-up or restart.
                // For now, we block until it's ready.
                return Ok(None);
            }

            // Tie-break: If the error resource is Tier 3 (Application), proactively scale to 0 if it's not already.
            // (Assuming Namespace-based tier mapping for the demo)
            if error.resource.namespace == "llm-apps" || error.resource.namespace == "orchestrators"
            {
                println!(
                    "[Workflow] Boot Storm: Suspending Tier 3 resource {} (namespace: {})",
                    error.resource.name, error.resource.namespace
                );
                self.remediator.pause_resource(&error.resource).await?;
                return Ok(None);
            }

            // Sequential Release Logic
            // In a real implementation, this would be a background loop.
            // Here we check dependencies and release if tiers 1 & 2 are ready.
            if let Some(dep) = self
                .remediator
                .check_startup_dependency(&error.resource)
                .await?
            {
                println!(
                    "[Workflow] Tiered dependency block: {} is waiting for {}.",
                    error.resource.name, dep
                );
                self.remediator.pause_resource(&error.resource).await?;
                return Ok(None);
            }

            // If we reach here, dependencies are ready. Resume.
            self.remediator.resume_resource(&error.resource).await?;
            return Ok(None);
        }

        if matches!(
            startup_state.phase,
            StartupPhase::Initial | StartupPhase::InProcess
        ) {
            // ... (Legacy reactive logic remains as fallback)
            let mut attempts = 0;
            let max_wait_attempts = 10;

            while let Some(dep) = self
                .remediator
                .check_startup_dependency(&error.resource)
                .await?
            {
                if attempts == 0 {
                    println!(
                        "[Workflow] Detected startup dependency: {} is waiting for {}. Pausing...",
                        error.resource.name, dep
                    );
                    self.remediator.pause_resource(&error.resource).await?;
                }

                if attempts >= max_wait_attempts {
                    println!(
                        "[Workflow] Timeout waiting for dependency '{}' for resource '{}'. Resuming anyway to avoid deadlock.",
                        dep, error.resource.name
                    );
                    break;
                }

                println!(
                    "[Workflow] Still waiting for dependency '{}' (Attempt {}/{})...",
                    dep,
                    attempts + 1,
                    max_wait_attempts
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                attempts += 1;
            }

            if attempts > 0 {
                println!(
                    "[Workflow] Dependency resolved or timeout reached. Resuming resource '{}'...",
                    error.resource.name
                );
                self.remediator.resume_resource(&error.resource).await?;

                // Stability Phase: Wait 15s and verify health to ensure no immediate restart
                tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
                if self.remediator.verify_resource(&error.resource).await? {
                    println!(
                        "[Workflow] Resource '{}' started successfully without restarts.",
                        error.resource.name
                    );
                } else {
                    println!(
                        "[Workflow] Warning: Resource '{}' is still unhealthy after resumption.",
                        error.resource.name
                    );
                }

                return Ok(None);
            }
        }

        let mut proposal = self.remediator.propose_fix(&error).await?;
        let tracking_id = proposal.tracking_id;
        let mut attempts = 1;
        let max_attempts = 3;

        loop {
            println!(
                "[Workflow] Attempt {} for context {}",
                attempts,
                &tracking_id.to_string()[..8]
            );

            // Security Check
            SecurityValidator::validate_proposal(&proposal)?;

            // Execution
            let outcome = self.remediator.execute_fix(&proposal).await?;

            // Verification
            let is_healthy = self.remediator.verify_resource(&error.resource).await?;

            if is_healthy {
                println!("[Workflow] Resource is healthy after attempt {}.", attempts);

                // If there's a permanent code change proposed, commit it now.
                if !proposal.code_change.is_empty() {
                    println!("[Workflow] Creating GitOps PR for verified solution...");
                    self.remediator.create_gitops_pr(&proposal).await?;
                }

                return Ok(Some(outcome));
            }

            if attempts >= max_attempts {
                println!(
                    "[Workflow] Max attempts reached for context {}.",
                    &tracking_id.to_string()[..8]
                );
                return Ok(Some(outcome));
            }

            // Failure: Provide feedback to Jules
            let feedback = format!(
                "Command '{}' executed but resource is still unhealthy. Logs: {}",
                proposal.remediation_command.as_deref().unwrap_or("none"),
                outcome.logs
            );

            println!("[Workflow] Feedback to AI: {}", feedback);
            proposal = self.remediator.refine_fix(tracking_id, &feedback).await?;
            attempts += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::services::MockRemediator;
    use chrono::Utc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_handle_error_success() {
        let mut mock = MockRemediator::new();
        let error_id = Uuid::new_v4();
        let proposal_id = Uuid::new_v4();
        let tracking_id = Uuid::new_v4();

        // Classification
        mock.expect_classify_error().returning(|_| true);

        // Startup State (Normal)
        mock.expect_get_startup_state().returning(|| {
            Ok(ClusterStartupState {
                phase: StartupPhase::Stabilized,
                event_count: 100,
                start_time: Utc::now(),
                current_tier: DependencyTier::Applications,
                boot_storm_detected: false,
                batch_size: 2,
                release_interval_secs: 60,
            })
        });

        // Proposal
        mock.expect_propose_fix().returning(move |_| {
            Ok(FixProposal {
                error_id,
                proposal_id,
                tracking_id,
                code_change: "".into(),
                explanation: "".into(),
                risk_score: RiskScore::Low,
                confidence: 1.0,
                remediation_command: Some("kubectl patch deployment foo".into()),
            })
        });

        // Execution
        mock.expect_execute_fix().returning(move |_| {
            Ok(RemediationOutcome {
                proposal_id,
                tracking_id,
                success: true,
                latency_ms: 100,
                logs: "Success".into(),
            })
        });

        // Verification
        mock.expect_verify_resource().returning(|_| Ok(true));

        let workflow = RemediationWorkflow::new(Arc::new(mock));
        let error = ClusterError {
            id: error_id,
            timestamp: Utc::now(),
            severity: Severity::Medium,
            error_type: ErrorType::Structural,
            resource: ClusterResource {
                kind: "Pod".into(),
                name: "foo".into(),
                namespace: "default".into(),
                api_version: "v1".into(),
            },
            message: "Failed".into(),
            error_code: "OOMKilled".into(),
            raw_event: serde_json::Value::Null,
        };

        let result = workflow.handle_error(error).await;
        assert!(result.is_ok());
        let outcome = result.unwrap();
        assert!(outcome.is_some());
        assert!(outcome.unwrap().success);
    }

    #[tokio::test]
    async fn test_startup_orchestration_pause() {
        let mut mock = MockRemediator::new();
        let error_id = Uuid::new_v4();

        mock.expect_classify_error().returning(|_| true);

        // Simulate Initial Startup
        mock.expect_get_startup_state().returning(|| {
            Ok(ClusterStartupState {
                phase: StartupPhase::Initial,
                event_count: 2,
                start_time: Utc::now(),
                current_tier: DependencyTier::Bootstrap,
                boot_storm_detected: false,
                batch_size: 2,
                release_interval_secs: 60,
            })
        });

        // Detect dependency: Return Some first, then None to break loop
        let mut dep_results = vec![Ok(None), Ok(Some("db".into()))];
        mock.expect_check_startup_dependency()
            .returning(move |_| dep_results.pop().unwrap());

        // Expect pause, resume and verify
        mock.expect_pause_resource().once().returning(|_| Ok(()));
        mock.expect_resume_resource().once().returning(|_| Ok(()));
        mock.expect_verify_resource().once().returning(|_| Ok(true));

        let workflow = RemediationWorkflow::new(Arc::new(mock));
        let error = ClusterError {
            id: error_id,
            timestamp: Utc::now(),
            severity: Severity::Medium,
            error_type: ErrorType::Structural,
            resource: ClusterResource {
                kind: "Pod".into(),
                name: "app".into(),
                namespace: "default".into(),
                api_version: "v1".into(),
            },
            message: "Failed".into(),
            error_code: "BackOff".into(),
            raw_event: serde_json::Value::Null,
        };

        let result = workflow.handle_error(error).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
