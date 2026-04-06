# Project Aethelgard 🏰

**The Self-Healing Dark System for MicroK8s.**

Project Aethelgard is a high-performance, autonomous remediation system designed for memory-constrained Kubernetes environments. It leverages a Rust-native core (**ZeroClaw**) to monitor cluster health and utilizes **Google Jules (via MCP)** to perform intelligent, GitOps-driven error remediation.

## 🛰️ Mission

To eliminate manual intervention in Kubernetes maintenance by transitioning to an autonomous, event-driven infrastructure that observes, diagnoses, and repairs itself in real-time.

## 🏗️ Architecture: The Dark System

Aethelgard operates as a "Dark System," silently maintaining the cluster's stability by reconciling the "Source of Truth" in Git.

- **The Watcher (Rust)**: High-speed observation via `kube-rs`.
- **The Orchestrator (ZeroClaw)**: Rust-based MCP host for AI coordination.
- **The Executor (Jules MCP)**: Autonomous AI engineering agent.
- **The Sync (FluxCD)**: Automated GitOps reconciliation.
- `src/application/`: Orchestration and use case handling.
- `src/infrastructure/`: Kubernetes and Jules SDK integrations.
- `src/interface/`: Webhook entry points for FluxCD.
- `ml/`: Experiment tracking and model metrics via MLflow.
- `k8s/`: GitOps manifests for Deployment and RBAC.
- `.artifacts/`: Historical planning and execution docs.

## Getting Started

1.  **Install Dependencies**:
    ```bash
    poetry install
    ```
2.  **Run the Webhook**:
    ```bash
    uvicorn src.interface.webhook:app --host 0.0.0.0 --port 8000
    ```
3.  **Configure FluxCD**: Point your FluxCD alerts to the `/webhook/flux-alert` endpoint.

## Contribution for Agents

If you are an AI agent working on this repository, please read the [AGENTS.md](file:///mnt/F024B17C24B145FE/Repos/Jules_remediator/AGENTS.md) and follow the [Remediation Process Workflow](file:///mnt/F024B17C24B145FE/Repos/Jules_remediator/_agent/workflows/remediation-process.md).
