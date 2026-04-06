import mlflow
import os

class MLflowTracker:
    def __init__(self, experiment_name: str = "jules-remediator-factory"):
        # Configure MLflow tracking
        self.experiment_name = experiment_name
        self.tracking_uri = os.getenv("MLFLOW_TRACKING_URI", "http://localhost:5000")
        
        mlflow.set_tracking_uri(self.tracking_uri)
        
        # Check if experiment exists, otherwise create it
        experiment = mlflow.get_experiment_by_name(self.experiment_name)
        if experiment is None:
            mlflow.create_experiment(self.experiment_name)
        
        mlflow.set_experiment(self.experiment_name)

    def log_remediation_event(self, event_id: str, severity: str, success: bool, latency: float):
        with mlflow.start_run(run_name=f"event-{event_id}"):
            mlflow.log_params({
                "event_id": event_id,
                "severity": severity,
                "success": success
            })
            mlflow.log_metrics({
                "remediation_success": 1.0 if success else 0.0,
                "remediation_latency": latency
            })
            
    def get_experiment_id(self):
        return mlflow.get_experiment_by_name(self.experiment_name).experiment_id
