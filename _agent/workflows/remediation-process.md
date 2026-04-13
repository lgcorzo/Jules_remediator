---
description: How to follow the Jules Remediator process for cluster error remediation.
---

# Remediation Process Workflow (Rust Implementation)

This document explains the step-by-step process used by the Jules Remediator to detect, classify, and fix Kubernetes errors.

## Workflow Overview

1.  **Alert Ingestion**:
    - The `Remediator` agent watches for Kubernetes Events via `K8sWatcher`.
    - It detects `Warning` events (e.g., BackOff, FailedScheduling, OOMKilled) and `Normal` startup events.

2.  **Error Classification**:
    - The watcher maps the incoming event to a `ClusterError` domain model.
    - The `RemediationWorkflow` evaluates the error's severity and decides if automated remediation is warranted.

3.  **Startup Orchestration**:
    - If the cluster is in a startup phase, the remediator evaluates dependencies.
    - Resources might be paused (scaled to 0) if their dependencies are not yet healthy.

4.  **Jules Session Trigger**:
    - The `RemediationWorkflow` uses `JulesDispatcher` (via JSON-RPC) to start an AI session.
    - The AI is provided with the error context and the current resource state.

5.  **Proposal Evaluation**:
    - Jules generates a `FixProposal` (containing commands or code changes).
    - The `SecurityValidator` ensures no malicious commands reach the cluster.

6.  **Fix Application**:
    - The agent applies the chosen fix.
    - This can involve direct patching of the cluster or creating a Pull Request in the GitOps repository.

7.  **Closure and Observation**:
    - Outcomes are persisted to `surreal.db` for historical analysis and learning.
    - Metrics are tracked to ensure zero restarts and high remediation success.

## Implementation Details

- **Domain Logic**: Defined in `src/domain/models.rs` and `src/domain/services.rs`.
- **Infrastructure**: Handles K8s API (`k8s_watcher.rs`), persistence (`persistence.rs`), and AI dispatching (`jules_dispatcher.rs`).
- **Persistence**: Uses **SurrealDB** for state and conversation history.
