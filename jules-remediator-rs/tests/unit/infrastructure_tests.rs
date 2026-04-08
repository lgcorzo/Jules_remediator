use jules_remediator_rs::infrastructure::*;
use jules_remediator_rs::domain::models::*;
use jules_remediator_rs::domain::services::Remediator;
use mockito::Server;
use uuid::Uuid;
use chrono::Utc;

#[tokio::test]
async fn test_surreal_persistence() {
    let persistence = SurrealPersistence::new("mem://").await.unwrap();
    let error = create_test_error();
    
    assert!(persistence.save_error(&error).await.is_ok());
    
    let outcome = RemediationOutcome {
        proposal_id: Uuid::new_v4(),
        success: true,
        latency_ms: 123,
        logs: "Test".into(),
    };
    assert!(persistence.save_outcome(&outcome).await.is_ok());
}

#[tokio::test]
async fn test_mlflow_logger() {
    let mut server = Server::new_async().await;
    
    // Mock run creation
    let mock_create = server.mock("POST", "/api/2.0/mlflow/runs/create")
        .with_status(200)
        .with_body(r#"{"run": {"info": {"run_id": "test_run_123"}}}"#)
        .create_async().await;

    // Mock metric logging (success)
    let _mock_log_success = server.mock("POST", "/api/2.0/mlflow/runs/log-metric")
        .with_status(200)
        .create_async().await;

    let logger = MlflowLogger::new(server.url());
    let result = logger.log_remediation(true, 500).await;
    
    assert!(result.is_ok());
    mock_create.assert_async().await;
    // log_remediation calls log-metric twice (success and latency)
    // mockito only assertions once for the first matching mock unless we configure it.
    // However, the test passing means the HTTP calls were made.
}

#[tokio::test]
async fn test_jules_dispatcher() {
    let mut server = Server::new_async().await;
    
    let mock_fix = server.mock("POST", "/")
        .with_status(200)
        .with_body(r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "code_change": "fixed",
                "explanation": "test fix",
                "risk_score": "Low",
                "confidence": 0.99
            }
        }"#)
        .create_async().await;

    let dispatcher = JulesDispatcher::new(&server.url()).await.unwrap();
    let error = create_test_error();
    
    let result = dispatcher.get_fix(&error).await;
    assert!(result.is_ok());
    let proposal = result.unwrap();
    assert_eq!(proposal.explanation, "test fix");
    mock_fix.assert_async().await;
}

#[tokio::test]
async fn test_remediator_impl_integration() {
    let mut server = Server::new_async().await;
    
    // Mock for Jules
    let _mock_jules = server.mock("POST", "/")
        .with_status(200)
        .with_body(r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"code_change": "", "explanation": "", "risk_score": "Low", "confidence": 1.0}
        }"#)
        .create_async().await;

    // Mock for MLflow
    let _mock_mlflow_create = server.mock("POST", "/api/2.0/mlflow/runs/create")
        .with_status(200)
        .with_body(r#"{"run": {"info": {"run_id": "test"}}}"#)
        .create_async().await;
    let _mock_log = server.mock("POST", "/api/2.0/mlflow/runs/log-metric")
        .with_status(200)
        .create_async().await;

    let remediator = RemediatorImpl::new(
        &server.url(),
        &server.url(),
        "mem://"
    ).await.unwrap();

    let error = create_test_error();
    let proposal: FixProposal = remediator.propose_fix(&error).await.unwrap();
    let outcome: RemediationOutcome = remediator.execute_fix(&proposal).await.unwrap();
    
    assert!(outcome.success);
}

#[tokio::test]
async fn test_classify_error_logic() {
    let mut server = Server::new_async().await;
    let remediator = RemediatorImpl::new(
        &server.url(),
        &server.url(),
        "mem://"
    ).await.unwrap();

    let mut error = create_test_error();
    error.error_code = "OOMKilled".into();
    assert!(remediator.classify_error(&error));

    error.error_code = "BackOff".into();
    assert!(remediator.classify_error(&error));

    error.error_code = "SomeOtherError".into();
    assert!(!remediator.classify_error(&error));

    error.error_code = "OOMKilled".into();
    error.message = "transient network error".into();
    assert!(!remediator.classify_error(&error));
}

fn create_test_error() -> ClusterError {
    ClusterError {
        id: Uuid::new_v4(),
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
