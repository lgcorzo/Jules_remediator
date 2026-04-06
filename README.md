# Jules Remediator Factory

Autonomous Kubernetes error remediation factory powered by Jules AI.

## Overview

The Jules Remediator is a self-healing system that listens for FluxCD alerts and automatically proposes and applies fixes to cluster errors. It adheres to **Domain-Driven Design (DDD)** and **MLOps 2026** standards.

## Project Structure

- `src/domain/`: Core business logic and error models.
- `src/application/`: Orchestration and use case handling.
- `src/infrastructure/`: Kubernetes and Jules SDK integrations.
- `src/interface/`: Webhook entry points for FluxCD.
- `ml/`: Experiment tracking and model metrics via MLflow.
- `k8s/`: GitOps manifests for Deployment and RBAC.
- `_agent/workflows/`: Standard operating procedures for AI agents.
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
