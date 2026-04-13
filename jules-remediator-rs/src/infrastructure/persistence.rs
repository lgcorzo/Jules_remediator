use crate::domain::models::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

pub struct SurrealPersistence {
    errors: RwLock<HashMap<Uuid, ClusterError>>,
    outcomes: RwLock<HashMap<Uuid, RemediationOutcome>>,
    messages: RwLock<Vec<ConversationMessage>>,
    steps: RwLock<Vec<RemediationStep>>,
    startup_events: RwLock<Vec<StartupEvent>>,
}

impl SurrealPersistence {
    pub async fn new(_path: &str) -> Result<Self> {
        Ok(Self {
            errors: RwLock::new(HashMap::new()),
            outcomes: RwLock::new(HashMap::new()),
            messages: RwLock::new(Vec::new()),
            steps: RwLock::new(Vec::new()),
            startup_events: RwLock::new(Vec::new()),
        })
    }

    pub async fn save_error(&self, error: &ClusterError) -> Result<()> {
        let mut errors = self.errors.write().unwrap();
        errors.insert(error.id, error.clone());
        Ok(())
    }

    pub async fn save_outcome(&self, outcome: &RemediationOutcome) -> Result<()> {
        let mut outcomes = self.outcomes.write().unwrap();
        outcomes.insert(outcome.proposal_id, outcome.clone());
        Ok(())
    }

    pub async fn save_message(&self, message: &ConversationMessage) -> Result<()> {
        let mut messages = self.messages.write().unwrap();
        messages.push(message.clone());
        Ok(())
    }

    pub async fn get_history(&self, session_id: Uuid) -> Result<Vec<ConversationMessage>> {
        let messages = self.messages.read().unwrap();
        Ok(messages
            .iter()
            .filter(|m| m.session_id == session_id)
            .cloned()
            .collect())
    }

    pub async fn save_step(&self, step: &RemediationStep) -> Result<()> {
        let mut steps = self.steps.write().unwrap();
        steps.push(step.clone());
        Ok(())
    }

    pub async fn save_startup_event(&self, event: &StartupEvent) -> Result<()> {
        let mut events = self.startup_events.write().unwrap();
        events.push(event.clone());
        Ok(())
    }

    pub async fn get_startup_timeline(&self) -> Result<Vec<StartupEvent>> {
        let events = self.startup_events.read().unwrap();
        let mut sorted = events.clone();
        sorted.sort_by_key(|e| e.timestamp);
        Ok(sorted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_new() {
        let persistence = SurrealPersistence::new("mem://").await;
        assert!(persistence.is_ok());
    }

    #[tokio::test]
    async fn test_save_error() {
        let persistence = SurrealPersistence::new("mem://").await.unwrap();
        let error = ClusterError {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            severity: Severity::High,
            error_type: ErrorType::Structural,
            resource: ClusterResource {
                kind: "Pod".into(),
                name: "test-pod".into(),
                namespace: "default".into(),
                api_version: "v1".into(),
            },
            message: "Test error".into(),
            error_code: "OOMKilled".into(),
            raw_event: serde_json::Value::Null,
        };

        let result = persistence.save_error(&error).await;
        assert!(result.is_ok());

        let errors = persistence.errors.read().unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors.get(&error.id).unwrap().message, "Test error");
    }

    #[tokio::test]
    async fn test_save_outcome() {
        let persistence = SurrealPersistence::new("mem://").await.unwrap();
        let outcome = RemediationOutcome {
            proposal_id: Uuid::new_v4(),
            success: true,
            latency_ms: 100,
            logs: "Test logs".into(),
        };

        let result = persistence.save_outcome(&outcome).await;
        assert!(result.is_ok());

        let outcomes = persistence.outcomes.read().unwrap();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(
            outcomes.get(&outcome.proposal_id).unwrap().logs,
            "Test logs"
        );
    }
}
