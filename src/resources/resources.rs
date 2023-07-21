use std::time::Duration;
use tokio::time::sleep;
use tracing::*;

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{Api, DeleteParams, ResourceExt, PatchParams},
    Client,
};
use async_trait::async_trait;

#[async_trait]
pub trait Resource: Send + Sync{
    fn client(&self) -> Client;
    fn name(&self) -> String;
    fn group(&self) -> String;
    fn version(&self) -> String;
    async fn delete(&self) -> anyhow::Result<()>{
        let client = self.client();
        let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
        let fqdn = format!("{}.{}", self.name(), self.group());
        let dp = DeleteParams::default();
        let _ = crds.get(fqdn.as_str()).await.map(|res|{
            let pp = PatchParams::default();
        });
        let _ = crds.delete(fqdn.as_str(), &dp).await.map(|res| {
            res.map_left(|o| {
                info!(
                    "Deleting {}: ({:?})",
                    o.name_any(),
                    o.status.unwrap().conditions.unwrap().last()
                );
            })
            .map_right(|s| {
                info!("Deleted foos.clux.dev: ({:?})", s);
            })
        });
        sleep(Duration::from_secs(2)).await;
        Ok(())
    }
    async fn create(&self) -> anyhow::Result<()>;
}

pub async fn init_resources(resource_list: Vec<Box<dyn Resource>>) -> anyhow::Result<()>{
    for resource in &resource_list{
        resource.delete().await?;
    }
    for resource in &resource_list{
        resource.create().await?;
    }
    Ok(())
}