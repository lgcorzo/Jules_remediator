use crate::domain::models::*;
use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::engine::local::{Mem, SurrealKv};
use uuid::Uuid;

pub struct SurrealPersistence {
    db: Surreal<surrealdb::engine::local::Db>,
}

impl SurrealPersistence {
    pub async fn new(path: &str) -> Result<Self> {
        let db = if path == "mem://" || path.is_empty() {
            let db = Surreal::new::<Mem>(()).await?;
            db.use_ns("jules").use_db("remediator").await?;
            db
        } else {
            let db = Surreal::new::<SurrealKv>(path).await?;
            db.use_ns("jules").use_db("remediator").await?;
            db
        };

        Ok(Self { db })
    }

    pub async fn save_error(&self, error: &ClusterError) -> Result<()> {
        let _: Option<ClusterError> = self
            .db
            .update(("errors", error.id.to_string()))
            .content(error.clone())
            .await?;
        Ok(())
    }

    pub async fn save_outcome(&self, outcome: &RemediationOutcome) -> Result<()> {
        let _: Option<RemediationOutcome> = self
            .db
            .update(("outcomes", outcome.proposal_id.to_string()))
            .content(outcome.clone())
            .await?;
        Ok(())
    }

    pub async fn save_message(&self, message: &ConversationMessage) -> Result<()> {
        let _: Option<ConversationMessage> =
            self.db.create("messages").content(message.clone()).await?;
        Ok(())
    }

    pub async fn get_messages(&self, tracking_id: Uuid) -> Result<Vec<ConversationMessage>> {
        let messages: Vec<ConversationMessage> = self.db.select("messages").await?;

        Ok(messages
            .into_iter()
            .filter(|m: &ConversationMessage| m.tracking_id == tracking_id)
            .collect())
    }

    pub async fn save_step(&self, step: &RemediationStep) -> Result<()> {
        let _: Option<RemediationStep> = self.db.create("steps").content(step.clone()).await?;
        Ok(())
    }

    pub async fn save_startup_event(&self, event: &StartupEvent) -> Result<()> {
        let _: Option<StartupEvent> = self
            .db
            .create("startup_events")
            .content(event.clone())
            .await?;
        Ok(())
    }

    pub async fn get_startup_timeline(&self) -> Result<Vec<StartupEvent>> {
        let mut events: Vec<StartupEvent> = self.db.select("startup_events").await?;
        events.sort_by_key(|e| e.timestamp);
        Ok(events)
    }

    /// Identifies resources that historically have high restart counts or slow startup.
    pub async fn get_unstable_resources(&self) -> Result<Vec<ClusterResource>> {
        let events = self.get_startup_timeline().await?;
        let mut unstable = Vec::new();

        // Heuristic: If a resource appears in the timeline more than 3 times as "Started"
        // before a "Ready" event in the same session, it's considered unstable.
        // For now, we return unique resources that have multiple "Started" events.
        let mut counts = std::collections::HashMap::new();
        for event in events {
            if event.status == "Started" {
                *counts.entry(event.resource.name.clone()).or_insert(0) += 1;
                if counts[&event.resource.name] > 2
                    && !unstable
                        .iter()
                        .any(|r: &ClusterResource| r.name == event.resource.name)
                {
                    unstable.push(event.resource.clone());
                }
            }
        }

        Ok(unstable)
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

        persistence.save_error(&error).await.unwrap();
    }

    #[tokio::test]
    async fn test_conversation_history() {
        let persistence = SurrealPersistence::new("mem").await.unwrap();
        let tracking_id = Uuid::new_v4();

        persistence
            .save_message(&ConversationMessage {
                tracking_id,
                timestamp: chrono::Utc::now(),
                role: "jules".into(),
                content: "Hello".into(),
            })
            .await
            .unwrap();

        let history = persistence.get_messages(tracking_id).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, "Hello");
    }
}
