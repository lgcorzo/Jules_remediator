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
        println!("[Remediator] Processing event: {}", error.id);

        if !self.remediator.classify_error(&error) {
            println!(
                "[Remediator] Error {} classified as non-remediable or transient.",
                error.id
            );
            return Ok(None);
        }

        println!("[Remediator] Proposing fix for error: {}", error.id);
        let proposal = self
            .remediator
            .propose_fix(&error)
            .await
            .context("failed to propose fix")?;

        // --- Security Layer ---
        SecurityValidator::validate_proposal(&proposal)
            .context("security check failed for proposal")?;

        println!("[Remediator] Executing fix: {}", proposal.proposal_id);
        let outcome = self
            .remediator
            .execute_fix(&proposal)
            .await
            .context("failed to execute fix")?;

        if outcome.success {
            println!(
                "[Remediator] Success! Error {} remediated in {}ms",
                error.id, outcome.latency_ms
            );
        } else {
            println!(
                "[Remediator] Failure: Error {} remediation failed.",
                error.id
            );
        }

        Ok(Some(outcome))
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
