use crate::domain::models::{ClusterError, FixProposal};
use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use std::env;

/// ZeroClaw is the Orchestrator that bridges the K8s cluster and the Jules AI via MCP.
/// It implements the Host side of the Model Context Protocol over STDIO.
pub struct ZeroClaw {
    binary_path: String,
    api_key: String,
}

impl ZeroClaw {
    pub fn new() -> Result<Self> {
        let binary_path = env::var("JULES_CLI_PATH").unwrap_or_else(|_| "jules-cli".into());
        let api_key = env::var("JULES_API_KEY").context("JULES_API_KEY environment variable not set")?;

        Ok(Self {
            binary_path,
            api_key,
        })
    }

    /// Primary orchestration method: sends an error to Jules and receives a fix proposal.
    pub async fn orchestrate_remediation(&self, error: &ClusterError) -> Result<FixProposal> {
        println!("[ZeroClaw] Orchestrating remediation for error: {}", error.id);

        // Prepare the prompt for Jules
        let prompt = format!(
            "Analyze and fix this Kubernetes error:\n\nResource: {}/{}\nError Code: {}\nMessage: {}\n",
            error.namespace, error.resource_name, error.error_code, error.message
        );

        // In Phase 2, we use a simplified STDIO execution for jules-cli.
        // In the future, this will evolve into a persistent MCP Session via mcp-rust-sdk.
        let output = Command::new(&self.binary_path)
            .arg("mcp")
            .arg("fix")
            .arg("--prompt")
            .arg(prompt)
            .env("JULES_API_KEY", &self.api_key)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to execute jules-cli")?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Jules AI error: {}", err_msg));
        }

        let response: FixProposal = serde_json::from_slice(&output.stdout)
            .context("Failed to parse fix proposal from Jules AI")?;

        Ok(response)
    }
}
