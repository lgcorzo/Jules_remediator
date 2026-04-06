# Project Requirements: Jules Remediator 📋

The following requirements define the core capabilities and performance standards for the autonomous remediation system.

## 🟢 Functional Requirements

### 1. Alert Ingestion (Webhook)
- **Support**: Must receive and parse JSON alerts from the FluxCD controller.
- **Filtering**: Ignore low-severity or informational-only alerts.

### 2. Error Classification
- **Domain Modeling**: Map incoming alerts to the `ClusterError` domain model.
- **Severity Scoring**: Automatically assign a severity level (`Low`, `Medium`, `High`, `Critical`).

### 3. AI-Driven Remediation (Jules SDK)
- **Context Ingestion**: Provide the Jules SDK with the current resource YAML and the error message.
- **Fix Proposal**: Generate at least one `FixProposal` containing a valid remediation command or code change.

### 4. Safety Guardrails
- **Validation**: All proposals must pass the `SecurityValidator` (command sanitization and prefix tracking).
- **Risk Assessment**: High-risk changes must generate a Pull Request instead of a direct cluster patch.

### 5. Observability (MLflow)
- **Tracking**: Log every remediation session as an MLflow run.
- **Success Metrics**: Record `remediation_success` and `latency_ms`.

---

## 🟡 Non-Functional Requirements

| ID | Category | Requirement | Target |
| :--- | :--- | :--- | :--- |
| **NFR-1** | Performance | Time from alert ingestion to fix proposal. | < 20s |
| **NFR-2** | Accuracy | Remediation success rate for known patterns. | > 90% |
| **NFR-3** | Safety | Number of bypasses of the `SecurityValidator`. | 0 |
| **NFR-4** | Availability | Remediator Pod uptime. | 99.9% |
| **NFR-5** | Scalability | Capacity to handle simultaneous alerts. | > 50 / min |

## 🛠️ Tech Stack & Constraints
- **Language**: Rust (for high-performance and safety).
- **Core SDK**: Jules SDK (Google DeepMind).
- **Platform**: Kubernetes (Cluster-native execution).
- **GitOps**: Compatibility with FluxCD and Git repositories.

---

> [!NOTE]
> Future expansions will include **Human-in-the-loop (HITL)** approval gates and **Multi-step remediation** logic.
