use garde::Validate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::*;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{Api, PostParams, ResourceExt},
    core::crd::CustomResourceExt,
    Client, CustomResource,
};
use async_trait::async_trait;
use k8s_openapi::api::core::v1 as core_v1;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta_v1;
use std::fmt::{Display, Result, Formatter};

use crate::resources::resources::Resource;

use super::resources;

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub enum AddressFamily{
    Inet,
    InetLabeled,
    InetVpn,
    Evpn,
    RouteTarget,
    Inet6,
    Inet6Vpn,
}

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, Validate, JsonSchema)]
#[kube(group = "cnm.juniper.net", version = "v1", kind = "BgpRouter", namespaced)]
#[kube(status = "BgpRouterStatus")]
#[serde(rename_all = "camelCase")]
#[kube(printcolumn = r#"{"name":"ASN", "jsonPath": ".spec.autonomousSystemNumber", "type": "integer"}"#)]
#[kube(printcolumn = r#"{"name":"RouterId", "jsonPath": ".spec.routerId", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"Address", "jsonPath": ".spec.address", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"Type", "jsonPath": ".spec.routerType", "type": "string"}"#)]
pub struct BgpRouterSpec {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    pub autonomous_system_number: i32,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub router_id: Option<String>,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v4_address: Option<String>,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v6_address: Option<String>,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,
    #[garde(skip)]
    pub address_families: Vec<AddressFamily>,
    #[garde(skip)]
    pub managed: bool,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bgp_peer_references: Option<Vec<BgpPeeringReference>>,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_parent: Option<resources::Parent>,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_instance_parent: Option<core_v1::ObjectReference>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BgpPeeringReference{
    pub peer_reference: core_v1::ObjectReference,
    pub session_attributes: BgpSessionAttributes,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bgp_router_group: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BgpSessionAttributes{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_v4_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_v4_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_v6_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_v6_address: Option<String>,
    pub local_as: i32,
    pub peer_as: i32,
    pub address_families: Vec<AddressFamily>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BgpRouterStatus {
    pub bgp_peer_references: Option<Vec<BgpPeeringReference>>,
}

pub struct BgpRouterResource{
    client: Client,
    name: String,
    group: String,
    version: String,
}

impl BgpRouterResource{
    pub fn new(client: Client) -> Self{
        let name = "bgprouters".to_string();
        let group = "cnm.juniper.net".to_string();
        let version = "v1".to_string();
        BgpRouterResource{
            client,
            name,
            group,
            version,
        }
    }
}

#[async_trait]
impl Resource for BgpRouterResource{
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
        let crd = BgpRouter::crd();
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
        sleep(Duration::from_secs(1)).await;
        Ok(())
    }
}
