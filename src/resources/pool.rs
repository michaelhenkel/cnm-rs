use garde::Validate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{time::Duration, collections::{HashSet, BTreeMap, BTreeSet}};
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
use crate::controllers::crpd::junos::routing_instance::Instance;

use crate::resources::resources::Resource;
use k8s_openapi::api::core::v1 as core_v1;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, Validate, JsonSchema)]
#[kube(group = "cnm.juniper.net", version = "v1", kind = "Pool", namespaced)]
#[kube(status = "PoolStatus")]
#[serde(rename_all = "camelCase")]
//#[kube(printcolumn = r#"{"name":"Team", "jsonPath": ".spec.metadata.team", "type": "string"}"#)]
pub struct PoolSpec {
    #[garde(skip)]
    pub pool_type: PoolType,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PoolType{
    V4{
        prefix: String,
        length: u8,
    },
    V6{
        prefix: String,
        length: u8,
    },
    RouteTarget{
        start: u32,
        size: u32,
    },
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoolStatus {
    pub in_use: u128,
    pub max_size: u128,
    pub length: u8,
    pub next_available: u128,
    pub released_numbers: BTreeMap<u128, Option<bool>>,
}

impl PoolStatus{
    pub fn assign_number(&mut self) -> Option<u128> {
        //let (ip, _) = self.released_numbers.iter().next_back()?;
        let ip = match self.released_numbers.iter().next_back(){
            Some((ip, _)) => { Some(*ip) },
            None => None
        };
        if let Some(ip) = ip{
            self.released_numbers.remove(&ip);
            self.in_use += 1;
            return Some(ip);
        }
        self.in_use += 1;
        self.next_available += 1;
        Some(self.next_available)
    }
    pub fn return_number(&mut self, number: u128) {
        // Clear the bit corresponding to the returned number in the bitmask
        self.in_use -=1;
        self.released_numbers.insert(number, None);
    }
}

pub struct PoolResource{
    client: Client,
    name: String,
    group: String,
    version: String,
}

impl PoolResource{
    pub fn new(client: Client) -> Self{
        let name = "pools".to_string();
        let group = "cnm.juniper.net".to_string();
        let version = "v1".to_string();
        PoolResource{
            client,
            name,
            group,
            version,
        }
    }
}

#[async_trait]
impl Resource for PoolResource{
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
        let crd = Pool::crd();
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
