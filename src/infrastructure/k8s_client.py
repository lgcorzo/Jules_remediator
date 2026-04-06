from kubernetes import client, config
from src.domain.models import ClusterResource

class K8sClient:
    def __init__(self):
        try:
            config.load_incluster_config()
        except config.ConfigException:
            config.load_kube_config()
        self.apps_v1 = client.AppsV1Api()
        self.core_v1 = client.CoreV1Api()

    def get_resource_yaml(self, resource: ClusterResource) -> str:
        # Example: Fetch the manifest of a resource
        if resource.kind.lower() == "deployment":
            api_response = self.apps_v1.read_namespaced_deployment(
                name=resource.name, namespace=resource.namespace
            )
        elif resource.kind.lower() == "pod":
            api_response = self.core_v1.read_namespaced_pod(
                name=resource.name, namespace=resource.namespace
            )
        else:
            raise NotImplementedError(f"Resource kind {resource.kind} not supported yet.")
        
        # In a real scenario, we'd serialize the response to YAML
        return str(api_response)

    def patch_resource(self, resource: ClusterResource, patch_body: dict) -> bool:
        if resource.kind.lower() == "deployment":
            self.apps_v1.patch_namespaced_deployment(
                name=resource.name, namespace=resource.namespace, body=patch_body
            )
            return True
        return False
