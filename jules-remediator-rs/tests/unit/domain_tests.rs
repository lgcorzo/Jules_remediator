use chrono::Utc;
use jules_remediator_rs::domain::models::*;
use jules_remediator_rs::domain::security::SecurityValidator;
use uuid::Uuid;

#[test]
fn test_severity_serialization() {
    let severity = Severity::High;
    let json = serde_json::to_string(&severity).unwrap();
    assert_eq!(json, "\"high\"");

    let deserialized: Severity = serde_json::from_str("\"high\"").unwrap();
    assert_eq!(deserialized, Severity::High);
}

#[test]
fn test_cluster_error_serialization() {
    let error = ClusterError {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        severity: Severity::Critical,
        error_type: ErrorType::Unknown,
        resource: ClusterResource {
            kind: "Deployment".into(),
            name: "api-server".into(),
            namespace: "prod".into(),
            api_version: "apps/v1".into(),
        },
        message: "OOMKilled".into(),
        error_code: "137".into(),
        raw_event: serde_json::json!({"reason": "OOMKilled"}),
    };

    let json = serde_json::to_string(&error).unwrap();
    let deserialized: ClusterError = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, error.id);
    assert_eq!(deserialized.severity, Severity::Critical);
}

#[test]
fn test_security_validator_safe_command() {
    let proposal = FixProposal {
        error_id: Uuid::new_v4(),
        proposal_id: Uuid::new_v4(),
        code_change: "spec.replicas = 2".into(),
        explanation: "Scaling up".into(),
        risk_score: RiskScore::Low,
        confidence: 0.95,
        remediation_command: Some(
            "kubectl patch deployment foo -p '{\"spec\":{\"replicas\":2}}'".into(),
        ),
    };

    assert!(SecurityValidator::validate_proposal(&proposal).is_ok());
}

#[test]
fn test_security_validator_restricted_chars() {
    let restricted_chars = ["&&", ";", "|", ">", "<", "$(", "`"];

    for char in restricted_chars {
        let proposal = FixProposal {
            error_id: Uuid::new_v4(),
            proposal_id: Uuid::new_v4(),
            code_change: "".into(),
            explanation: "".into(),
            risk_score: RiskScore::High,
            confidence: 0.1,
            remediation_command: Some(format!("kubectl patch deployment foo {}", char)),
        };
        assert!(
            SecurityValidator::validate_proposal(&proposal).is_err(),
            "Should catch restricted char: {}",
            char
        );
    }
}

#[test]
fn test_security_validator_invalid_prefix() {
    let proposal = FixProposal {
        error_id: Uuid::new_v4(),
        proposal_id: Uuid::new_v4(),
        code_change: "".into(),
        explanation: "".into(),
        risk_score: RiskScore::Low,
        confidence: 0.1,
        remediation_command: Some("rm -rf /".into()),
    };

    let result = SecurityValidator::validate_proposal(&proposal);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("prefix not in safe list")
    );
}
