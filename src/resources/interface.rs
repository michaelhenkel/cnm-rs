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


use crate::resources::resources::Resource;


use super::resources;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, Validate, JsonSchema, Default)]
#[kube(group = "cnm.juniper.net", version = "v1", kind = "Interface", namespaced)]
#[kube(status = "InterfaceStatus")]
#[serde(rename_all = "camelCase")]
//#[kube(printcolumn = r#"{"name":"Team", "jsonPath": ".spec.metadata.team", "type": "string"}"#)]
pub struct InterfaceSpec {
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
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
    pub instance_parent: resources::Parent,

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


#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct InterfaceStatus {}

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
