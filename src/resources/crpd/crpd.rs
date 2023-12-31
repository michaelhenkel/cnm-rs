use garde::Validate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{time::Duration, collections::BTreeMap};
use tokio::time::sleep;
use tracing::*;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{Api, PostParams, ResourceExt},
    core::crd::CustomResourceExt,
    Client, CustomResource,
};
use async_trait::async_trait;
use k8s_openapi::api::apps::v1 as apps_v1;
use k8s_openapi::Metadata;
use kube::api::ObjectMeta;
use std::collections::HashMap;

use crate::resources::resources::Resource;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, Validate, JsonSchema)]
#[kube(group = "cnm.juniper.net", version = "v1", kind = "Crpd", namespaced)]
#[kube(status = "CrpdStatus")]
#[serde(rename_all = "camelCase")]
//#[kube(printcolumn = r#"{"name":"Team", "jsonPath": ".spec.metadata.team", "type": "string"}"#)]
pub struct CrpdSpec {
    #[garde(skip)]
    pub replicas: i32,
    #[garde(skip)]
    pub image: String,
    #[garde(skip)]
    pub init_image: String,
}


#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct CrpdStatus {
    pub stateful_set: Option<apps_v1::StatefulSetStatus>,
    pub instances: Option<Vec<Instance>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct Instance{
    pub name: String,
    pub address: String,
    pub uuid: String,
}

pub struct CrpdResource{
    client: Client,
    name: String,
    group: String,
    version: String,
}


impl CrpdResource{
    pub fn new(client: Client) -> Self{
        let name = "crpds".to_string();
        let group = "cnm.juniper.net".to_string();
        let version = "v1".to_string();
        CrpdResource{
            client,
            name,
            group,
            version,
        }
    }
}

#[async_trait]
impl Resource for CrpdResource{
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
        let crd = Crpd::crd();
        info!("Creating CRD: {}",self.name);
        let pp = PostParams::default();
        match crds.create(&pp, &crd).await {
            Ok(o) => {
                info!("Created {}", o.name_any());
            }
            Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409), // if you skipped delete, for instance
            Err(e) => return Err(e.into()),                        // any other case is probably bad
        }
        sleep(Duration::from_secs(1)).await;
        Ok(())
    }
}
