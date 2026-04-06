import time
import mlflow
from src.domain.models import ClusterError, Severity
from src.domain.services import RemediationService
from src.infrastructure.k8s_client import K8sClient
from src.infrastructure.jules_sdk import JulesIntegration

class RemediationOrchestrator:
    def __init__(
        self,
        service: RemediationService,
        k8s_client: K8sClient,
        jules: JulesIntegration
    ):
        self.service = service
        self.k8s_client = k8s_client
        self.jules = jules

    async def handle_error(self, error: ClusterError):
        start_time = time.time()
        
        # Start MLflow experiment run
        with mlflow.start_run(run_name=f"error-fix-{error.id}"):
            mlflow.log_params({
                "error_id": error.id,
                "severity": error.severity,
                "kind": error.resource.kind,
                "name": error.resource.name
            })
            
            if not self.service.should_trigger_remediation(error):
                print(f"Skipping remediation for low severity error: {error.id}")
                mlflow.log_metric("remediation_skipped", 1)
                return

            try:
                # Trigger Jules for fix proposals
                proposals = await self.jules.get_fix_proposals(error)
                best_proposal = self.service.select_best_proposal(proposals)
                
                if best_proposal:
                    mlflow.log_param("proposal_id", best_proposal.proposal_id)
                    mlflow.log_metric("confidence", best_proposal.confidence)
                    
                    success = await self.jules.apply_fix(best_proposal)
                    
                    mlflow.log_metric("remediation_success", 1 if success else 0)
                else:
                    mlflow.log_metric("remediation_failed_no_proposals", 1)
                    
            except Exception as e:
                mlflow.log_metric("remediation_error", 1)
                mlflow.set_tag("error", str(e))
                raise
            finally:
                latency = time.time() - start_time
                mlflow.log_metric("latency_seconds", latency)
