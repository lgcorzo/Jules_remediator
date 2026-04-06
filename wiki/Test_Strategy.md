# Test Strategy & Quality Plan 🧪

To ensure the reliability of autonomous remediation, we implement a multi-layered testing strategy combining standard software engineering practices with AI-specific evaluation.

## 🧱 Testing Layers

| Level | Goal | Tech Stack |
| :--- | :--- | :--- |
| **Unit Tests** | Logic validation (models, services). | `cargo test` + `mockall`. |
| **Integration Tests** | Real-world scenario simulation. | `tests/` integration folder. |
| **LLM Eval** | Fix quality assessment. | **Jules SDK Eval** + **MLflow**. |
| **Security Tests** | Guardrail validation. | `SecurityValidator` unit tests. |

## 🛡️ Unit Testing (Rust)

We use `mockall` to simulate infrastructure ports (e.g., `IK8sClient`, `IMlflowTracker`).
- **Example**: `cargo test domain::security::tests` to verify command injection blocks.
- **Example**: `cargo test application::remediation::tests` to verify the state machine flow.

## 🔗 Integration Testing

Simulate FluxCD alerts and observe the Remediator's response without a live cluster.
1. **Mock Webhook**: Send JSON alerts via `curl`.
2. **Mock K8s API**: Use a mock server or specific test manifests in `k8s/base`.
3. **Verify Outcome**: Ensure the correct `FixProposal` is generated.

## 🔬 LLM Evaluation & Feedback

Unlike traditional code, the LLM's output is non-deterministic.
- **Dataset**: A curated set of 20+ common Kubernetes error scenarios (CrashLoop, OOMKilled, etc.).
- **Scoring**:
    - **Correctness**: Does the fix actually solve the error?
    - **Risk**: Is the `RiskScore` accurately predicted?
    - **Latency**: Is the inference fast enough for production?

### 📈 Metrics Tracking in MLflow

We track the **Success Rate** over time. If a specific error code consistently leads to failed remediations, it triggers an "LLM Re-training / Prompt Update" event.

---

> [!TIP]
> Before every release, run the full integration suite to ensure that no new security guardrails or architectural changes break the current remediation baseline.
