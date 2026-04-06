# Security Strategy & Safety Plan 🛡️

The **Jules Remediator** is designed with a "Safety First" architecture to ensure that autonomous actions never compromise cluster stability or security.

## 🧱 Layered Defense

| Layer | Responsibility | Implementation |
| :--- | :--- | :--- |
| **RBAC** | Least Privilege | Defined in `k8s/base/remediator-rbac.yaml`. |
| **Validation** | Command Sanitization | `SecurityValidator` in `domain/security.rs`. |
| **Audit** | Transparency | Every action is logged to **MLflow** with its `FixProposal`. |
| **Approvals** | Human-in-the-loop | PR creation for high-risk changes. |

## 🛡️ SecurityValidator Detail

The **SecurityValidator** performs a strict check on every `FixProposal` before execution.

### Anti-Injection Checks
Any command containing the following shell metacharacters is immediately **BLOCKED**:
- `&&`, `;`, `|`, `>`, `<`, `$()`, `` ` ``, `\`

### Restricted Executables
Only a predefined set of "Safe Tooling" is allowed as a command prefix:
- `kubectl patch`
- `kubectl label`
- `kubectl annotate`
- `kubectl rollout`

> [!CAUTION]
> If a proposal includes `rm -rf`, `delete`, or `curl`, it will be flagged as a **Security Violation** and logged to MLflow.

## 🔐 Identity & Access Management (IAM)

The Remediator Pod runs as a dedicated ServiceAccount.
- **Namespaced Scope**: Limited to specifically designated namespaces (e.g., `prod`, `qa`).
- **Secret Management**: We use **Sealed Secrets** to handle API keys and Git credentials.

## 🚨 Incident Response for Agents

If the model predicts a `RiskScore::High`, the system is configured to **Never Patch Directly**. Instead:
1. **Pull Request**: A PR is generated in the GitOps repository.
2. **Review Required**: An SRE must approve the change before FluxCD reconciles it.

---

> [!IMPORTANT]
> The security goal is to maintain **Zero Trust** in the LLM's raw output while leveraging its **High Logic Capability**.
