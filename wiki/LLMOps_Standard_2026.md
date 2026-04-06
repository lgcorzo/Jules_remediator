# LLMOps Standard 2026

In the **Jules Remediator** project, every remediation session is treated as a first-class experiment. We follow a strict **"Experiment-First"** approach to ensure the autonomous system learns from every interaction.

## 🧪 Experiment Lifecycle

1.  **Trigger**: An alert arrives from FluxCD.
2.  **Experiment Initialization**: A new MLflow "Run" is started.
3.  **Classification**: The LLM classifies the error type (OOM, CrashLoopBackOff, ConfigError).
4.  **Generation**: Jules generates a fix proposal.
5.  **Execution**: The fix is applied (PR or direct patch).
6.  **Observability**: The system waits for the cluster state to stabilize.
7.  **Closure**: The result is logged to MLflow, and the run is marked as `Success` or `Failed`.

## 📊 Metrics to Track

| Metric | Description | Goal |
| :--- | :--- | :--- |
| `remediation_success` | Binary (0 or 1) indicating if the fix worked. | > 95% |
| `latency_to_fix` | Time from alert reception to fix application. | < 5 mins |
| `token_usage` | LLM cost per remediation session. | Optimized |
| `resource_usage` | K8s resource consumption during remediation. | Minimal |

## 🔄 Feedback Loop
If a remediation fails, the system captures the logs and the "Failed" state. This data is used to fine-tune the prompts or provide better context for the next session.

> [!NOTE]
> All metrics are accessible via the internal MLflow dashboard.
