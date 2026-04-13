use crate::domain::models::*;
use crate::domain::security::SecurityValidator;
use crate::domain::services::Remediator;
use anyhow::{Context, Result};
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
        if matches!(startup_state.phase, StartupPhase::Initial | StartupPhase::InProcess) {
            if let Some(dep) = self.remediator.check_startup_dependency(&error.resource).await? {
                println!("[Workflow] Detected startup dependency: {} is waiting for {}. Pausing...", error.resource.name, dep);
                
                // Pause the resource to prevent restarts
                self.remediator.pause_resource(&error.resource).await?;
                
                // Wait for dependency (normally we'd use a more sophisticated wait/notify, 
                // but for now we'll just return and let another event trigger retry or simple wait)
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                
                // Resume and see if it works
                self.remediator.resume_resource(&error.resource).await?;
                return Ok(None); // Let the next event trigger logic if it still fails
            }
        }

        let mut proposal = self.remediator.propose_fix(&error).await?;
        let session_id = proposal.session_id;
        let mut attempts = 0;
        let max_attempts = 3;

        loop {
            attempts += 1;
            println!("[Workflow] Attempt {} for session {}", attempts, session_id);

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
                println!("[Workflow] Max attempts reached for session {}.", session_id);
                return Ok(Some(outcome));
            }

            // Failure: Provide feedback to Jules
            let feedback = format!(
                "Command '{}' executed but resource is still unhealthy.\nLogs:\n{}",
                proposal.remediation_command.clone().unwrap_or_else(|| "none".into()),
                outcome.logs
            );
            
            println!("[Workflow] Feedback to Jules: {}", feedback);
            proposal = self.remediator.refine_fix(session_id, &feedback).await?;
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

        // 1. Classification
        mock.expect_classify_error().returning(|_| true);

        // 2. Proposal
        mock.expect_propose_fix().returning(move |_| {
            Ok(FixProposal {
                error_id,
                proposal_id,
                code_change: "".into(),
                explanation: "".into(),
                risk_score: RiskScore::Low,
                confidence: 1.0,
                remediation_command: Some("kubectl patch deployment foo".into()),
            })
        });

        // 3. Execution
        mock.expect_execute_fix().returning(move |_| {
            Ok(RemediationOutcome {
                proposal_id,
                success: true,
                latency_ms: 100,
                logs: "Success".into(),
            })
        });

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
    async fn test_handle_error_security_fail() {
        let mut mock = MockRemediator::new();
        // Proposal with injection
        mock.expect_classify_error().returning(|_| true);
        mock.expect_propose_fix().returning(|_| {
            Ok(FixProposal {
                error_id: Uuid::new_v4(),
                proposal_id: Uuid::new_v4(),
                code_change: "".into(),
                explanation: "".into(),
                risk_score: RiskScore::Low,
                confidence: 1.0,
                remediation_command: Some("kubectl patch deployment foo; rm -rf /".into()),
            })
        });

        let workflow = RemediationWorkflow::new(Arc::new(mock));
        let error = ClusterError {
            id: Uuid::new_v4(),
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
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("security check failed")
        );
    }
}
