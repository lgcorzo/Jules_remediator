from fastapi import FastAPI, Request, HTTPException
from src.domain.models import ClusterError, Severity, ClusterResource
from src.domain.services import RemediationService, HighSeverityTrigger
from src.infrastructure.k8s_client import K8sClient
from src.infrastructure.jules_sdk import JulesIntegration
from src.application.orchestrator import RemediationOrchestrator
import uuid

app = FastAPI(title="Jules Remediator Webhook")

# Initialize DDD components
remediation_service = RemediationService(strategies=[HighSeverityTrigger()])
k8s_client = K8sClient()
jules_integration = JulesIntegration()
orchestrator = RemediationOrchestrator(
    service=remediation_service,
    k8s_client=k8s_client,
    jules=jules_integration
)

@app.post("/webhook/flux-alert")
async def receive_flux_alert(alert_data: dict):
    # Example FluxCD alert parsing
    # Map FluxCD alert to ClusterError model
    try:
        resource_info = alert_data.get("involvedObject", {})
        error_id = str(uuid.uuid4())
        
        error = ClusterError(
            id=error_id,
            severity=Severity.HIGH if alert_data.get("reason") in ["Failed", "Error"] else Severity.MEDIUM,
            resource=ClusterResource(
                kind=resource_info.get("kind", "Unknown"),
                name=resource_info.get("name", "Unknown"),
                namespace=resource_info.get("namespace", "default"),
                api_version=resource_info.get("apiVersion", "v1")
            ),
            message=alert_data.get("message", "No message provided"),
            error_code=alert_data.get("reason", "UnknownError"),
            raw_event=alert_data
        )
        
        await orchestrator.handle_error(error)
        
        return {"status": "accepted", "error_id": error_id}
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.get("/health")
def health_check():
    return {"status": "ok"}
