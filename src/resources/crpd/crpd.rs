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
use k8s_openapi::Metadata;
use kube::api::ObjectMeta;

use crate::resources::resources::Resource;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, Validate, JsonSchema)]
#[kube(group = "cnm.juniper.net", version = "v1", kind = "Crpd", namespaced)]
#[kube(status = "CrpdStatus")]
//#[kube(printcolumn = r#"{"name":"Team", "jsonPath": ".spec.metadata.team", "type": "string"}"#)]
pub struct CrpdSpec {
    #[garde(skip)]
    replicas: i32,
    #[garde(skip)]
    image: String,
}


#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct CrpdStatus {
    is_bad: bool,
    replicas: i32,
}

pub struct CrpdResource{
    client: Client,
    name: String,
    group: String,
    version: String,
}

/*
impl k8s_openapi::Metadata for Crpd {
    fn metadata(&self) -> &ObjectMeta {
        &self.metadata
    }
    fn metadata_mut(&mut self) -> &mut ObjectMeta {
        &mut self.metadata
    }
}
*/

/*
impl k8s_openapi::Resource for Crpd{
    const API_VERSION: &'static str = "cnm.juniper.net/v1";
    const GROUP: &'static str = "cnm.juniper.net";
    const KIND: &'static str = "Crpd";
    const VERSION: &'static str = "v1";
    type Scope = dyn k8s_openapi::ResourceScope;
}
*/


impl CrpdResource{
    pub fn new(client: Client) -> Self{
        let name = "crpd".to_string();
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
        // Wait for the api to catch up
        sleep(Duration::from_secs(1)).await;
        Ok(())
    }
}
