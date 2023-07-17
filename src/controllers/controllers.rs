use async_trait::async_trait;
use kube::{Client, Error, Api};
use k8s_openapi::NamespaceResourceScope;
use kube::api::ObjectMeta;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::sync::Arc;


#[derive(Debug)]
pub struct ReconcileError(pub anyhow::Error);
impl std::error::Error for ReconcileError {

}
impl std::fmt::Display for ReconcileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"ReconcileError: {}", self.0)
    }
}

#[async_trait]
pub trait Controller: Send + Sync{
    async fn run(&self) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct Context {
    pub client: Client,
}

pub async fn init_controllers(controller_list: Vec<Box<dyn Controller>>) -> anyhow::Result<()>{
    let mut handles = Vec::new();
    for controller in controller_list {
        let handle = tokio::spawn(async move {
            controller.run().await
        });
        handles.push(handle);
    }
    futures::future::join_all(handles).await;
    Ok(())
}

pub fn is_not_found(e: &Error) -> bool {
    match e{
        kube::Error::Api(ae) => {
            match ae{
                kube::error::ErrorResponse { status: s, message, reason, code } => {
                    match s.as_str(){
                        "NotFound" => {
                            return true
                        },
                        _ => {
                            return false
                        },
                    }
                },
                _ => {
                    return false
                },
            }
        },
        _ => {
            return false
        },
    }
}

//Result<Option<T>, ReconcileError>

pub async fn get_resource<T: kube::Resource>(t: Arc<T>, client: Client) -> Result<Option<(T,Api<T>)>, ReconcileError>
where
T: kube::Resource<Scope = NamespaceResourceScope>,
<T as kube::Resource>::DynamicType: Default,
T: Clone + DeserializeOwned + Debug,
{
    let res_api: Api<T> = Api::namespaced(client.clone(), t.meta().namespace.as_ref().unwrap());
    let res = match res_api.get(t.meta().name.as_ref().unwrap().as_str()).await{
        Ok(res) => {
            Some((res, res_api))
        },
        Err(e) => {
            if is_not_found(&e){
                None
            } else {
                return Err(ReconcileError(e.into()));
            }
        },
    };
    Ok(res)
}
