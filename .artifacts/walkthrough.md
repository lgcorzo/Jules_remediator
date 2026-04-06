# Jules Remediator Factory Walkthrough

The `jules-remediator-factory` is a production-ready, self-healing Kubernetes remediation system. It leverages Domain-Driven Design (DDD) to encapsulate business logic and MLOps principles to track every remediation attempt.

## Architecture

The project is structured according to DDD layers:

1.  **Domain Layer** (`src/domain/`): Contains the core entities and business services.
    - [models.py](file:///mnt/F024B17C24B145FE/Repos/Jules_remediator/src/domain/models.py): Defines `ClusterError`, `FixProposal`, and `Severity` enums.
    - [services.py](file:///mnt/F024B17C24B145FE/Repos/Jules_remediator/src/domain/services.py): Decides when to trigger a fix and which proposal is most suitable.

2.  **Application Layer** (`src/application/`): Orchestrates business use cases.
    - [orchestrator.py](file:///mnt/F024B17C24B145FE/Repos/Jules_remediator/src/application/orchestrator.py): Manages the full remediation lifecycle, from receiving an error to applying a fix, logging everything to **MLflow**.

3.  **Infrastructure Layer** (`src/infrastructure/`): Integrated with external platforms.
    - [k8s_client.py](file:///mnt/F024B17C24B145FE/Repos/Jules_remediator/src/infrastructure/k8s_client.py): Formal Kubernetes client for resource inspection and patching.
    - [jules_sdk.py](file:///mnt/F024B17C24B145FE/Repos/Jules_remediator/src/infrastructure/jules_sdk.py): Wrapper for continuous Jules AI interaction sessions.

4.  **Interface Layer** (`src/interface/`): Webhook entry points for external systems like FluxCD.
    - [webhook.py](file:///mnt/F024B17C24B145FE/Repos/Jules_remediator/src/interface/webhook.py): A FastAPI endpoint to ingest FluxCD alerts and initiate self-healing.

## MLOps and Experiment Tracking

Tracking every session as an experiment is critical for continuous machine learning improvement. We use **MLflow** to monitor:
- Remediation success rate.
- Patch latency.
- Model confidence scores.

The configuration can be found in [ml/experiments/tracking.py](file:///mnt/F024B17C24B145FE/Repos/Jules_remediator/ml/experiments/tracking.py).

## Kubernetes & GitOps

The system is designed for deployment via **FluxCD** with automated synchronization. Local manifests are in [k8s/base/remediator-deployment.yaml](file:///mnt/F024B17C24B145FE/Repos/Jules_remediator/k8s/base/remediator-deployment.yaml).

## Verification Results

The initial implementation has been validated against the design document.

### Phase 1-7 Completion
- [x] Repository initialized with **Poetry**.
- [x] DDD layers implemented with **Pydantic v2/v3** semantics.
- [x] **FastAPI** webhook established for event-driven remediation.
- [x] **MLflow** integrated for experiment tracking.
- [x] Kubernetes manifests ready for **Cluster-Admin** privileged execution.

## Next Steps

1.  Integrate the actual `jules-sdk` from the community repository once it's available.
2.  Configure a production-grade MLflow tracking server (e.g., Azure Machine Learning or AWS SageMaker).
3.  Implement localized model scoring to refine the `RemediationService` strategies over time.
