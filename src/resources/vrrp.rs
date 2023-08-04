use garde::Validate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::*;

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta_v1;
use kube::{
    api::{Api, PostParams, ResourceExt},
    core::crd::CustomResourceExt,
    Client, CustomResource,
};
use async_trait::async_trait;


use crate::resources::resources::Resource;
use k8s_openapi::api::core::v1 as core_v1;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, Validate, JsonSchema)]
#[kube(group = "cnm.juniper.net", version = "v1", kind = "Vrrp", namespaced)]
#[kube(status = "VrrpStatus")]
#[serde(rename_all = "camelCase")]
//#[kube(printcolumn = r#"{"name":"Team", "jsonPath": ".spec.metadata.team", "type": "string"}"#)]
pub struct VrrpSpec {
    #[garde(skip)]
    pub interface_selector: InterfaceSelector,
    #[garde(skip)]
    pub group: u8,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_interval: Option<u8>,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<Track>,
    #[garde(skip)]
    pub virtual_address: VirtualAddress,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unicast: Option<VrrpUnicast>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum InterfaceSelector{
    Selector(meta_v1::LabelSelector),
    InterfaceGroupParent(core_v1::LocalObjectReference),
    Device(String)
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Track{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<Vec<TrackInterface>>,
    pub notify_master: String,
    pub notify_backup: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrackInterface{
    pub interface: String,
    pub weight_cost: u8,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VirtualAddress{
    pub address: VirtualAddressAdress,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum VirtualAddressAdress{
    Address(String),
    PoolReference(core_v1::LocalObjectReference),
}


#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VrrpUnicast{
    pub local_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_address: Option<Vec<String>>,
}


#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VrrpStatus {
    pub virtual_address: String,
}

pub struct VrrpResource{
    client: Client,
    name: String,
    group: String,
    version: String,
}

impl VrrpResource{
    pub fn new(client: Client) -> Self{
        let name = "vrrps".to_string();
        let group = "cnm.juniper.net".to_string();
        let version = "v1".to_string();
        VrrpResource{
            client,
            name,
            group,
            version,
        }
    }
}

#[async_trait]
impl Resource for VrrpResource{
    fn client(&self) -> Client{
        self.client.clone()
    }
    fn name(&self) -> String{
        self.name.clone()
    }
    fn group(&self) -> String {
        self.group.clone()
    }
    fn version(&self) -> String {
        self.version.clone()
    }
    async fn create(&self) -> anyhow::Result<()>{
        let crds: Api<CustomResourceDefinition> = Api::all(self.client.clone());
        let crd = Vrrp::crd();
        info!("Creating CRD: {}",self.name);
        let pp = PostParams::default();
        match crds.create(&pp, &crd).await {
            Ok(o) => {
                info!("Created {}", o.name_any());
            }
            Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409), // if you skipped delete, for instance
            Err(e) => return Err(e.into()),                        // any other case is probably bad
        }
        // Wait for the api to catch up
        sleep(Duration::from_millis(500)).await;
        Ok(())
    }
}
