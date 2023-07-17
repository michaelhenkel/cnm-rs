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

use crate::resources::resources::Resource;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, Validate, JsonSchema)]
#[kube(group = "cnm.juniper.net", version = "v1", kind = "RoutingInstance", namespaced)]
#[kube(status = "RoutingInstanceStatus")]
//#[kube(printcolumn = r#"{"name":"Team", "jsonPath": ".spec.metadata.team", "type": "string"}"#)]
pub struct RoutingInstanceSpec {
    #[schemars(length(min = 1))]
    #[garde(length(min = 3))]
    info: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct RoutingInstanceStatus {
    is_bad: bool,
    replicas: i32,
}

pub struct RoutingInstanceResource{
    client: Client,
    name: String,
    group: String,
    version: String,
}

impl RoutingInstanceResource{
    pub fn new(client: Client) -> Self{
        let name = "routinginstances".to_string();
        let group = "cnm.juniper.net".to_string();
        let version = "v1".to_string();
        RoutingInstanceResource{
            client,
            name,
            group,
            version,
        }
    }
}

#[async_trait]
impl Resource for RoutingInstanceResource{
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
        let crd = RoutingInstance::crd();
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
