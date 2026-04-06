# LLMOps Process 🚀

The **Jules Remediator** follows a strict **MLOps Standard 2026** approach where every autonomous remediation session is treated as an experiment.

## 🔬 Experiment Lifecycle

### 1. Trigger & Context Ingestion
When a FluxCD alert is received, the Remediator captures the full cluster state and initiates an **MLflow** run.
- **Run Name**: `remediation-uuid`
- **Parameters**: `resource_kind`, `error_code`, `git_ref`.

### 2. AI Inference (Jules Session)
The Jules SDK is triggered to propose a fix. We track:
- **Model**: `gemini-2.0-flash` (standard)
- **Prompt Version**: `v1.2.0-remediator`
- **Inference Latency**: Logged as `inference_ms`.

### 3. Execution & Evaluation
The proposed fix is validated by the `SecurityValidator` and then applied.
- **Metrics**: 
    - `remediation_success`: (0 or 1) based on the cluster's recovery status.
    - `latency_ms`: Total time from alert to fix application.
    - `risk_score`: As predicted by the LLM.

## 🔄 Feedback Loop

If a remediation fails, the `RemediationOutcome` logs are stored in MLflow. These failure cases are used to:
1. **Refine Prompting**: Adjust the `AGENTS.md` instructions for better classification.
2. **Backtesting**: Re-run the error scenario through a newer model or prompt.

## 📊 Monitoring Dashboard

| Metric | Target | Description |
| :--- | :--- | :--- |
| **Success Rate** | > 85% | Percentage of alerts successfully remediated. |
| **Latency** | < 45s | End-to-end remediation time. |
| **Safety Violation** | 0 | Number of commands blocked by `SecurityValidator`. |

> [!TIP]
> Use the MLflow UI to compare different model performances across `CrashLoopBackOff` scenarios.
