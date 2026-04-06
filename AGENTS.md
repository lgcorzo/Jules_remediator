# AGENTS.md

Instructions for **Jules** and **Antigravity** to work on the `jules-remediator` factory.

## Vision

This repository is an autonomous factory for Kubernetes error remediation. The "Remediator Pod" listens for FluxCD alerts and triggers Jules to fix errors in the cluster.

## Rules for Agents

### 1. Domain-Driven Design (DDD)

- Always separate logic into `domain`, `application`, `infrastructure`, and `interface`.
- **Domain** layer must be independent of any frameworks or libraries.
- **Application** layer defines the use cases.
- **Infrastructure** layer handles technology specifics.
- **Interface** layer handles external communication (REST, CLI).

### 2. MLOps Standard 2026

- Every Jules session is an experiment.
- Log metrics to MLflow: `remediation_success`, `latency`, `resource_usage`.
- If remediation fails, log why and use it as feedback for future sessions.

### 3. Type Safety & Quality

- Use `Pydantic v2+` (or v3 when available) for all DTOs and entities.
- Enforce strict typing with `mypy`.
- Use `ruff` for linting and formatting.

### 4. Kubernetes Integration

- Use the formal `kubernetes-python-client`.
- Resource manifests should follow GitOps patterns (FluxCD).
- Deploy with `sealed-secrets` for sensitive data.

## Automation Workflow

1. **Alert Source:** FluxCD sends an alert to the Remediator webhook.
2. **Remediator:** Classifies the error and starts an MLflow "Run."
3. **Jules Trigger:** Remediator asks Jules for a fix via the Jules SDK.
4. **Fix Execution:** Jules creates a PR or directly patches the cluster.
5. **Observability:** Track the outcome and log it back to MLflow.

## Documentation for Other Agents

- **Standard Operating Procedure**: Read the [Remediation Process Workflow](file:///mnt/F024B17C24B145FE/Repos/Jules_remediator/_agent/workflows/remediation-process.md).
- **Historical Analysis**: Review historical plans, tasks, and walkthroughs in the [.artifacts/](file:///mnt/F024B17C24B145FE/Repos/Jules_remediator/.artifacts/) directory.
- **Metrics**: Access the MLflow dashboard for real-time experiment data.
