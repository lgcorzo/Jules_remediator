use crate::domain::models::*;
use crate::domain::services::Remediator;
use anyhow::Result;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

pub struct StartupMaster {
    remediator: Arc<dyn Remediator + Send + Sync>,
}

impl StartupMaster {
    pub fn new(remediator: Arc<dyn Remediator + Send + Sync>) -> Self {
        Self { remediator }
    }

    pub async fn run(&self) -> Result<()> {
        println!("[StartupMaster] Initializing Orchestration Loop...");

        loop {
            let state = self.remediator.get_startup_state().await?;

            if state.boot_storm_detected
                && matches!(
                    state.phase,
                    StartupPhase::Controlled | StartupPhase::Initial | StartupPhase::InProcess
                )
            {
                println!("[StartupMaster] Boot Storm detected! Entering Orchestration mode.");

                // 1. Proactive Lockdown
                self.lockdown().await?;

                // 2. Sequential Orchestration
                self.orchestrate(state).await?;

                println!("[StartupMaster] Orchestration complete. Cluster stabilized.");
            }

            sleep(Duration::from_secs(30)).await;
        }
    }

    async fn lockdown(&self) -> Result<()> {
        println!("[StartupMaster] Phase 0: Lockdown Tier 3 Resources.");
        let resources = self
            .remediator
            .get_tier_resources(DependencyTier::Applications)
            .await?;

        for res in resources {
            // Only pause if not already at 0 or unhealthy (simplification)
            self.remediator.pause_resource(&res).await?;
        }
        Ok(())
    }

    async fn orchestrate(&self, state: ClusterStartupState) -> Result<()> {
        // Step 1: Wait for Tier 0 (Bootstrap)
        println!("[StartupMaster] Phase 1: Waiting for Bootstrap (Tier 0) stability...");
        self.wait_for_tier(DependencyTier::Bootstrap).await?;

        // Step 2: Ensure Tier 1 (Foundations)
        println!("[StartupMaster] Phase 2: Releasing foundations (Tier 1)...");
        self.release_tier(DependencyTier::Foundation).await?;
        self.wait_for_tier(DependencyTier::Foundation).await?;

        // Step 3: Ensure Tier 2 (Core Services)
        println!("[StartupMaster] Phase 3: Releasing core services (Tier 2)...");
        self.release_tier(DependencyTier::CoreServices).await?;
        self.wait_for_tier(DependencyTier::CoreServices).await?;

        // Step 4: Batched release of Tier 3 (Applications)
        println!(
            "[StartupMaster] Phase 4: Releasing applications (Tier 3) in batches of {}...",
            state.batch_size
        );
        let tier3 = self
            .remediator
            .get_tier_resources(DependencyTier::Applications)
            .await?;

        for chunk in tier3.chunks(state.batch_size) {
            for res in chunk {
                self.remediator.resume_resource(res).await?;
            }
            println!(
                "[StartupMaster] Batch released. Waiting {}s for stabilization...",
                state.release_interval_secs
            );
            sleep(Duration::from_secs(state.release_interval_secs)).await;
        }

        Ok(())
    }

    async fn wait_for_tier(&self, tier: DependencyTier) -> Result<()> {
        loop {
            // Check readiness (0.0 to 1.0)
            // Note: We need a way to get progress. For now we use verify_resource on anchors.
            let resources = self.remediator.get_tier_resources(tier).await?;
            let mut all_ready = true;

            for res in resources {
                if !self.remediator.verify_resource(&res).await? {
                    all_ready = false;
                    break;
                }
            }

            if all_ready {
                break;
            }
            sleep(Duration::from_secs(10)).await;
        }
        Ok(())
    }

    async fn release_tier(&self, tier: DependencyTier) -> Result<()> {
        let resources = self.remediator.get_tier_resources(tier).await?;
        for res in resources {
            self.remediator.resume_resource(&res).await?;
        }
        Ok(())
    }
}
