use crate::unit::MockRemediator;
use chrono::Utc;
use jules_remediator_rs::application::RemediationWorkflow;
use jules_remediator_rs::domain::models::*;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_workflow_full_cycle() {
    let mut mock = MockRemediator::new();
    let error_id = Uuid::new_v4();
    let proposal_id = Uuid::new_v4();

    // 1. Classification
    mock.expect_classify_error().times(1).returning(|_| true);

    // 2. Proposal
    mock.expect_propose_fix().times(1).returning(move |_| {
        Ok(FixProposal {
            error_id,
            proposal_id,
            code_change: "fix".into(),
            explanation: "test".into(),
            risk_score: RiskScore::Low,
            confidence: 1.0,
            remediation_command: Some("kubectl patch deployment foo".into()),
        })
    });

    // 3. Execution
    mock.expect_execute_fix().times(1).returning(move |_| {
        Ok(RemediationOutcome {
            proposal_id,
            success: true,
            latency_ms: 100,
            logs: "Success".into(),
        })
    });

    let workflow = RemediationWorkflow::new(Arc::new(mock));
    let error = create_test_error(error_id);

    let result = workflow
        .handle_error(error)
        .await
        .expect("Error in handle_error");
    assert!(result.is_some());
    let outcome: RemediationOutcome = result.unwrap();
    assert!(outcome.success);
    assert_eq!(outcome.proposal_id, proposal_id);
}

#[tokio::test]
async fn test_workflow_skips_non_remediable() {
    let mut mock = MockRemediator::new();

    mock.expect_classify_error().times(1).returning(|_| false);

    let workflow = RemediationWorkflow::new(Arc::new(mock));
    let error = create_test_error(Uuid::new_v4());

    let result = workflow
        .handle_error(error)
        .await
        .expect("Error in handle_error");
    assert!(result.is_none());
}

fn create_test_error(id: Uuid) -> ClusterError {
    ClusterError {
        id,
        timestamp: Utc::now(),
        severity: Severity::Medium,
        resource: ClusterResource {
            kind: "Pod".into(),
            name: "test-pod".into(),
            namespace: "default".into(),
            api_version: "v1".into(),
        },
        message: "OOMKilled".into(),
        error_code: "OOMKilled".into(),
        raw_event: serde_json::Value::Null,
    }
}
