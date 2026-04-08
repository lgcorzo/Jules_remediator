# Remediation Workflow

The remediation process is a closed-loop automation flow between FluxCD, the Remediator Pod, Jules, and MLflow.

## 🛰️ Integration Flow

```mermaid
sequenceDiagram
    participant Cluster as K8s Cluster (FluxCD)
    participant RP as Remediator Pod
    participant ML as MLflow (Experiment Tracking)
    participant J as Jules (Agent)

    Cluster->>RP: FluxCD Alert (Webhook)
    RP->>ML: Start Experiment Run
    RP->>RP: Classify Error Context
    RP->>J: Request Fix (Remediation Prompt)
    J->>RP: Return Fix (Patch/PR)
    RP->>Cluster: Apply Fix
    RP->>RP: Wait for Health Check
    alt Fix Success
        RP->>ML: Log Success & Metrics
    else Fix Failed
        RP->>ML: Log Failure & Error Logs
    end
```

## 🛠️ Execution Details

### 1. Alert Classification

The Remediator parses the FluxCD alert payload and retrieves additional cluster context (logs, describe output) using the `kube-rs` client. 

**Categorization Logic**:

- **Transient Errors**: (e.g., `ErrImagePull` with network timeout) are logged but skipped to avoid redundant AI calls.
- **Permanent Errors**: (e.g., `OOMKilled`, `CrashLoopBackOff`, `InvalidConfig`) trigger the full Jules remediation loop.
- **Unknown Errors**: Default to remediation if severity is high.

### 2. Jules Interaction

The system constructs a detailed prompt for Jules, including:

- **Current Cluster State**
- **Error Logs**
- **Attempted Actions History**

### 3. Verification

After applying a fix, the pod monitors the resource's `READY` status for a configurable period (default: 300s) before confirming success.
