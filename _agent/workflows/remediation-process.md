---
description: How to follow the Jules Remediator process for cluster error remediation.
---

# Remediation Process Workflow

This document explains the step-by-step process used by the Jules Remediator to detect, classify, and fix Kubernetes errors.

## Workflow Overview

1.  **Alert Ingestion**:
    - The `Remediator Pod` exposes a FastAPI webhook at `/webhook/flux-alert`.
    - FluxCD sends a JSON payload to this endpoint when a resource (e.g., Kustomization, HelmRelease) fails to reconcile.

2.  **Error Classification**:
    - The webhook maps the incoming alert to a `ClusterError` domain model.
    - The `RemediationService` evaluates the error's severity and decides if automated remediation is warranted.

3.  **Experiment Initialization**:
    - An **MLflow** run is started to track the remediation session.
    - Initial parameters (error ID, resource kind, severity) are logged.

4.  **Jules Session Trigger**:
    - The `RemediationOrchestrator` uses the `JulesIntegration` (Jules SDK) to start an AI session.
    - The AI is provided with the error context and the current resource manifest from the cluster via the `K8sClient`.

5.  **Proposal Evaluation**:
    - Jules generates one or more `FixProposal` objects.
    - The `RemediationService` selects the best proposal based on confidence scores and risk assessment.

6.  **Fix Application**:
    - The orchestrator applies the chosen fix.
    - This can involve direct patching of the cluster via `K8sClient` or creating a Pull Request in the GitOps repository.

7.  **Closure and Observation**:
    - The outcome (success/failure) and latency are logged back to the MLflow run.
    - Future agents can review these logs to iterate on the `AGENTS.md` instructions or scoring logic.

## For Agents Reading the Repo

- **Domain Logic**: Check `src/domain/models.py` and `src/domain/services.py` for business rules.
- **Observability**: Check `ml/experiments/tracking.py` for telemetry patterns.
- **Mocking**: Use the `tests/` directory to simulate FluxCD alerts without affecting a live cluster.
