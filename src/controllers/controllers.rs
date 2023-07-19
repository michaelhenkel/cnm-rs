use async_trait::async_trait;
use kube::{Client, Error, Api};
use k8s_openapi::NamespaceResourceScope;
use kube::api::{ObjectMeta, PostParams};
use serde::de::DeserializeOwned;
use tracing::info;
use std::collections::BTreeMap;
use std::{fmt::Debug, borrow::BorrowMut};
use std::sync::Arc;
use serde::Serialize;
use kube::api::{Patch, PatchParams, ListParams, ObjectList};




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
                kube::error::ErrorResponse { status: _, message: _, reason: r, code: _ } => {
                    match r.as_str(){
                        "NotFound" => {
                            info!("Resource not found: {:?}", e);
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

pub fn already_exists(e: &Error) -> bool {
    match e{
        kube::Error::Api(ae) => {
            match ae{
                kube::error::ErrorResponse { status: _, message: _, reason: r, code: _ } => {
                    match r.as_str(){
                        "AlreadyExists" => {
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

pub async fn get<T: kube::Resource>(t: Arc<T>, client: Client) -> Result<Option<(T,Api<T>)>, ReconcileError>
where
T: kube::Resource<Scope = NamespaceResourceScope>,
<T as kube::Resource>::DynamicType: Default,
T: Clone + DeserializeOwned + Debug,
{
    let res_api: Api<T> = Api::namespaced(client.clone(), t.meta().namespace.as_ref().unwrap());
    let res = match res_api.get(t.meta().name.as_ref().unwrap().as_str()).await{
        Ok(res) => {
            info!("Found resource: {:?}", res.meta().name.as_ref().unwrap());
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

pub async fn list<T: kube::Resource>(namespace: &str, client: Client, labels: Option<BTreeMap<String, String>>) -> Result<Option<(ObjectList<T>,Api<T>)>, ReconcileError>
where
T: kube::Resource<Scope = NamespaceResourceScope>,
<T as kube::Resource>::DynamicType: Default,
T: Clone + DeserializeOwned + Debug,
{
    let res_api: Api<T> = Api::namespaced(client.clone(), namespace);
    let mut list_params = ListParams::default();
    for (k, v) in labels.unwrap().iter(){
        list_params.label_selector = Some(format!("{}={}", k, v));
    }
    let res = match res_api.list(&list_params).await{
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

pub async fn create<T: kube::Resource>(t: Arc<T>, client: Client) -> Result<Option<T>, ReconcileError>
where
T: kube::Resource<Scope = NamespaceResourceScope>,
<T as kube::Resource>::DynamicType: Default,
T: Clone + DeserializeOwned + Debug + Serialize,
{
    info!("Creating {:?}", t);
    let res_api: Api<T> = Api::namespaced(client.clone(), t.meta().namespace.as_ref().unwrap());
    let res = match res_api.create(&PostParams::default(), &t).await{
        Ok(res) => {
            Some(res)
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

pub async fn patch<T: kube::Resource>(t: T, client: Client) -> Result<Option<T>, ReconcileError>
where
T: kube::Resource<Scope = NamespaceResourceScope>,
<T as kube::Resource>::DynamicType: Default,
T: Clone + DeserializeOwned + Debug + Serialize,
{
    //info!("updating resource {:?}", t);
//t.borrow_mut().meta_mut().managed_fields = None;
    let patch = Patch::Merge(&t);
    let params = PatchParams::apply("crpd");
    let res_api: Api<T> = Api::namespaced(client.clone(), t.meta().namespace.as_ref().unwrap());
    let res = match res_api.patch(t.meta().name.as_ref().unwrap(), &params, &patch).await{
        Ok(res) => {
            Some(res)
        },
        Err(e) => {
            if is_not_found(&e){
                None
            } else {
                info!("Error updating resource: {:?}", t);
                return Err(ReconcileError(e.into()));
            }
        },
    };
    Ok(res)
}

pub async fn replace<T: kube::Resource>(mut t: T, client: Client) -> Result<Option<T>, ReconcileError>
where
T: kube::Resource<Scope = NamespaceResourceScope>,
<T as kube::Resource>::DynamicType: Default,
T: Clone + DeserializeOwned + Debug + Serialize,
{
    info!("Updating {:?}", t.meta().name.as_ref().unwrap());
    t.borrow_mut().meta_mut().managed_fields = None;
    let params = PostParams::default();
    let res_api: Api<T> = Api::namespaced(client.clone(), t.meta().namespace.as_ref().unwrap());
    let res = match res_api.replace(t.meta().name.as_ref().unwrap(), &params, &t).await{
        Ok(res) => {
            Some(res)
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

pub async fn create_or_update<T: kube::Resource>(t: T, client: Client) -> Result<Option<T>, ReconcileError>
where
T: kube::Resource<Scope = NamespaceResourceScope>,
<T as kube::Resource>::DynamicType: Default,
T: Clone + DeserializeOwned + Debug + Serialize,
{
    match get(Arc::new(t.clone()), client.clone()).await{
        Ok(res) => {
            match res{
                Some((mut current, _)) => {                    
                    patch(t, client).await
                },
                None => {
                    create(Arc::new(t), client).await
                },
            }
        },
        Err(e) => {
            Err(e)
        },
    }
}

pub async fn update_status<T: kube::Resource>(t: T, client: Client) -> Result<Option<T>, ReconcileError>
where
T: kube::Resource<Scope = NamespaceResourceScope>,
<T as kube::Resource>::DynamicType: Default,
T: Clone + DeserializeOwned + Debug + Serialize,
{
    info!("Updating Status {:?}", t.meta().name.as_ref().unwrap());
    let params = PatchParams::apply("crpd");
    let patch = Patch::Merge(t.clone());
    let res_api: Api<T> = Api::namespaced(client.clone(), t.meta().namespace.as_ref().unwrap());
    let res = match res_api.patch_status(t.clone().meta().name.as_ref().unwrap(), &params, &patch).await{
        Ok(res) => {
            Some(res)
        },
        Err(e) => {
            if is_not_found(&e){
                info!("status not found: {:?}", e);
                None
            } else {
                return Err(ReconcileError(e.into()));
            }
        },
    };
    Ok(res)
}
