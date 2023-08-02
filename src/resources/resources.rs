use std::{time::Duration, fmt::{Display, Result, Formatter}};
use tokio::time::sleep;
use tracing::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{Api, DeleteParams, ResourceExt, PatchParams},
    Client,
};
use async_trait::async_trait;
use k8s_openapi::api::core::v1 as core_v1;

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
        let _ = crds.get(fqdn.as_str()).await.map(|_res|{
            let _pp = PatchParams::default();
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
        sleep(Duration::from_millis(500)).await;
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

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Parent{
    pub parent_type: InstanceType,
    pub reference: core_v1::LocalObjectReference,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub enum InstanceType{
    Crpd,
    Generic,
    MetalLb,
    Tgw,
}

impl Default for Parent{
    fn default() -> Self{
        Parent{
            parent_type: InstanceType::Crpd,
            reference: core_v1::LocalObjectReference::default(),
        }
    }
}

impl Default for InstanceType{
    fn default() -> Self{
        InstanceType::Generic
    }
}

impl Display for InstanceType {
    fn fmt(&self, f: &mut Formatter) -> Result{
        match self {
            InstanceType::Crpd => write!(f, "Crpd"),
            InstanceType::Generic => write!(f, "Generic"),
            InstanceType::MetalLb => write!(f, "MetalLb"),
            InstanceType::Tgw => write!(f, "Tgw"),
        }
    }
}