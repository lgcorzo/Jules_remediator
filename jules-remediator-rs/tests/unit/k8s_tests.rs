use crate::unit::MockRemediator;
use jules_remediator_rs::application::RemediationWorkflow;
use jules_remediator_rs::infrastructure::K8sWatcher;
use k8s_openapi::api::core::v1::{Event, ObjectReference};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use std::sync::Arc;

#[tokio::test]
async fn test_handle_event_oomkilled() {
    let mut mock = MockRemediator::new();

    // We expect the workflow to be triggered for OOMKilled
    mock.expect_classify_error().returning(|_| true);
    mock.expect_propose_fix()
        .returning(|_| Err(anyhow::anyhow!("stop here"))); // Stop after proposal for this test

    let workflow = Arc::new(RemediationWorkflow::new(Arc::new(mock)));

    let event = Event {
        type_: Some("Warning".into()),
        reason: Some("OOMKilled".into()),
        involved_object: ObjectReference {
            kind: Some("Pod".into()),
            name: Some("test-pod".into()),
            namespace: Some("default".into()),
            ..Default::default()
        },
        metadata: ObjectMeta {
            name: Some("test-event".into()),
            ..Default::default()
        },
        message: Some("Memory limit reached".into()),
        ..Default::default()
    };

    let result = K8sWatcher::handle_event_logic(event, workflow).await;
    // Result should be Ok because error in handle_error is caught/logged but handle_event_logic returns Ok
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_ignore_normal_events() {
    let mock = MockRemediator::new();
    // No expectations on mock because it should NOT be called

    let workflow = Arc::new(RemediationWorkflow::new(Arc::new(mock)));

    let event = Event {
        type_: Some("Normal".into()),
        reason: Some("Scheduled".into()),
        ..Default::default()
    };

    let result = K8sWatcher::handle_event_logic(event, workflow).await;
    assert!(result.is_ok());
}
