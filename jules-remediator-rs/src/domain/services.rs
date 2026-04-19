use crate::domain::models::*;
use anyhow::Result;
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait] // Added async_trait for better trait compatibility
pub trait Remediator {
    /// Classifies an error event and determines if remediation is needed.
    fn classify_error(&self, error: &ClusterError) -> bool;

    /// Proposes a fix for a given error event.
    async fn propose_fix(&self, error: &ClusterError) -> Result<FixProposal>;

    /// Executes a remediation fix.
    async fn execute_fix(&self, proposal: &FixProposal) -> Result<RemediationOutcome>;

    /// Refines a fix based on feedback from a previous attempt.
    async fn refine_fix(&self, tracking_id: Uuid, feedback: &str) -> Result<FixProposal>;

    /// Verifies if the resource is healthy.
    async fn verify_resource(&self, resource: &ClusterResource) -> Result<bool>;

    /// Creates a PR in the GitOps repository for a verified solution.
    async fn create_gitops_pr(&self, proposal: &FixProposal) -> Result<()>;

    /// Gets the current startup state of the cluster.
    async fn get_startup_state(&self) -> Result<ClusterStartupState>;

    /// Pauses a resource (e.g. scale to 0) to wait for dependencies.
    async fn pause_resource(&self, resource: &ClusterResource) -> Result<()>;

    /// Resumes a resource (e.g. restore replicas).
    async fn resume_resource(&self, resource: &ClusterResource) -> Result<()>;

    /// Identifies if a resource is waiting for a dependency.
    async fn check_startup_dependency(&self, resource: &ClusterResource) -> Result<Option<String>>;

    /// Lists all manageable resources (Deployments/StatefulSets) in a namespace.
    async fn list_resources(&self, namespace: &str) -> Result<Vec<ClusterResource>>;

    /// Gets all resources defined in a specific tier mapping.
    async fn get_tier_resources(&self, tier: DependencyTier) -> Result<Vec<ClusterResource>>;

    /// Performs an autonomous analysis of the error using an LLM.
    async fn autonomous_review(&self, error: &ClusterError) -> Result<AutonomousReview>;

    /// Deletes failed pods in a namespace (or all namespaces if None) to reset restart counts.
    async fn delete_failed_pods(&self, namespace: Option<&str>) -> Result<usize>;
}
