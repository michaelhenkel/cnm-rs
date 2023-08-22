use garde::Validate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::*;
use std::str::FromStr;

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;

use kube::{
    api::{Api, PostParams, ResourceExt},
    core::crd::CustomResourceExt,
    Client, CustomResource,
};
use async_trait::async_trait;
use k8s_openapi::api::core::v1 as core_v1;

use crate::resources::resources::Resource;


use super::resources;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, Validate, JsonSchema, Default)]
#[kube(group = "cnm.juniper.net", version = "v1", kind = "Interface", namespaced)]
#[kube(status = "InterfaceStatus")]
#[serde(rename_all = "camelCase")]
//#[kube(printcolumn = r#"{"name":"Team", "jsonPath": ".spec.metadata.team", "type": "string"}"#)]
pub struct InterfaceSpec {
    #[garde(skip)]
    pub name: String,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtu: Option<u16>,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub families: Option<Vec<InterfaceFamily>>,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_parent: Option<resources::Parent>,
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vrrp: Option<Vrrp>,
    #[garde(skip)]
    pub managed: bool,
}
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum InterfaceFamily{
    Inet(InterfaceInet),
    Inet6(InterfaceInet6),
}

impl InterfaceFamily{
    pub fn new(address: &str) -> InterfaceFamily{
        let prefix = address.split("/").collect::<Vec<&str>>();
        let ip_addr = std::net::IpAddr::from_str(prefix[0]).unwrap();
        if ip_addr.is_ipv4(){
            let ipv4 = std::net::Ipv4Addr::from_str(prefix[0]).unwrap();
            InterfaceFamily::Inet(InterfaceInet{
                address: format!("{}/{}",ipv4.to_string(), prefix[1]),
            })
        } else {
            let ipv6 = std::net::Ipv6Addr::from_str(prefix[0]).unwrap();
            InterfaceFamily::Inet6(InterfaceInet6{
                address: format!("{}/{}",ipv6.to_string(), prefix[1]),
            })
        }
    }
}

impl Default for InterfaceFamily{
    fn default() -> Self{
        InterfaceFamily::Inet(InterfaceInet::default())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceInet{
    pub address: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceInet6{
    pub address: String,
}


#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct Vrrp{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_interval: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<Track>,
    pub virtual_address: VirtualAddress,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unicast: Option<VrrpUnicast>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v4_subnet_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v6_subnet_filter: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v4_address: Option<VirtualAddressAdress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v6_address: Option<VirtualAddressAdress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
}

impl Default for VirtualAddress{
    fn default() -> Self{
        VirtualAddress{
            v4_address: None,
            v6_address: None,
            device_name: None,
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_v4_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_v6_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_v4_list: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_v6_list: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vrrp: Option<Vrrp>,
}

pub struct InterfaceResource{
    client: Client,
    name: String,
    group: String,
    version: String,
}

impl InterfaceResource{
    pub fn new(client: Client) -> Self{
        let name = "interfaces".to_string();
        let group = "cnm.juniper.net".to_string();
        let version = "v1".to_string();
        InterfaceResource{
            client,
            name,
            group,
            version,
        }
    }
}

#[async_trait]
impl Resource for InterfaceResource{
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
        let crd = Interface::crd();
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
