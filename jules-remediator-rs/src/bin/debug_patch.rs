use kube::{Api, Client, api::{Patch, PatchParams}};
use k8s_openapi::api::apps::v1::Deployment;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("--- [Jules Diagnostic] SSA Logic Matcher ---");
    
    let client = Client::try_default().await?;
    let namespace = "llm-apps";
    let name = "docling-serve";
    let api_version = "apps/v1";
    let kind = "Deployment";
    
    let deployments: Api<Deployment> = Api::namespaced(client, namespace);

    println!("[Diagnostic] Attempting SSA with exact Remediator payload...");
    
    // This is the EXACT payload from remediator_impl.rs
    let patch = json!({
        "apiVersion": api_version,
        "kind": kind,
        "metadata": {
            "name": name,
            "namespace": namespace,
        },
        "spec": {
            "replicas": 0
        }
    });
    
    let params = PatchParams::apply("jules-remediator").force();
    
    match deployments.patch(name, &params, &Patch::Apply(patch)).await {
        Ok(_) => println!("✅ SUCCESS: SSA logic is correct and accepted by API!"),
        Err(e) => println!("❌ FAILURE: API rejected the exact payload: {:?}", e),
    }

    Ok(())
}
