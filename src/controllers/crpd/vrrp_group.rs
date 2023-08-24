use crate::controllers::controllers::{Controller, Context, ReconcileError};

use crate::controllers::controllers;
use crate::resources::interface::{Interface, InterfaceFamily};
use crate::resources::interface_group::InterfaceGroup;
use crate::resources::ip_address::{IpAddress, IpAddressSpec, IpFamily};
use crate::resources::pool::Pool;
use crate::resources::vrrp;
use crate::resources::vrrp_group::{
    VrrpGroup,
    VrrpGroupStatus
};
use crate::resources::{
    crpd::crpd,
    resources,
};
use garde::rules::ip;
use kube::{Resource, Client};
use async_trait::async_trait;
use futures::StreamExt;
use kube::{
    api::Api,
    runtime::{
        controller::{Action, Controller as runtime_controller},
        watcher::Config,
    },
};
use kube_runtime::reflector::ObjectRef;
use std::any;
use std::collections::{BTreeMap, HashMap};
use std::f32::consts::E;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;
use k8s_openapi::api::core::v1 as core_v1;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta_v1;

pub struct VrrpGroupController{
    context: Arc<Context>,
    resource: Api<VrrpGroup>,
}

impl VrrpGroupController{
    pub fn new(context: Arc<Context>) -> Self{
        let resource = Api::all(context.client.clone());
        
        VrrpGroupController{context, resource}
    }
    async fn reconcile(g: Arc<VrrpGroup>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        let name = g.meta().name.as_ref().unwrap();
        let namespace = g.meta().namespace.as_ref().unwrap();
        info!("reconciling VrrpGroup {:?}", g.meta().name.as_ref().unwrap().clone());
        
        let (mut vrrp_group, _vrrp_group_api) = match controllers::get::<VrrpGroup>(namespace, name,ctx.client.clone()).await{
            Ok(res) => {
                match res{
                    Some((interface, interface_api)) => {
                            (interface, interface_api)
                    }
                    None => { return Ok(Action::await_change()) }
                }
            },
            Err(e) => return Err(e)
        };
        
        let name = vrrp_group.meta().name.as_ref().unwrap();
        let namespace = vrrp_group.meta().namespace.as_ref().unwrap();

        match controllers::list::<vrrp::Vrrp>(namespace, ctx.client.clone(), Some(BTreeMap::from([
            ("cnm.juniper.net/vrrpGroup".to_string(), name.clone())
        ]))).await{
            Ok(res) => {
                if let Some((child_list,_)) = res {
                    let ref_list: Vec<core_v1::LocalObjectReference> = child_list.iter().map(|obj|{
                        core_v1::LocalObjectReference {
                            name: Some(obj.meta().name.as_ref().unwrap().clone()),
                        }
                    }).collect();
                    if ref_list.len() > 0 {
                        if let Some(status) = vrrp_group.status.as_mut(){
                            status.vrrp_references = Some(ref_list);
                        } else {
                            vrrp_group.status = Some(VrrpGroupStatus{
                                vrrp_references: Some(ref_list),
                            })
                        }
                        if let Err(e) = controllers::update_status(vrrp_group.clone(), ctx.client.clone()).await{
                            return Err(e);
                        }
                    }
                } 
            },
            Err(e) => return Err(e)
        }
        Ok(Action::await_change())
    }
    fn error_policy(_g: Arc<VrrpGroup>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}


#[async_trait]
impl Controller for VrrpGroupController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<VrrpGroup>, ctx: Arc<Context>| {
            async move { VrrpGroupController::reconcile(g, ctx).await }
        };
        let error_policy = |g: Arc<VrrpGroup>, error: &ReconcileError, ctx: Arc<Context>| {
            VrrpGroupController::error_policy(g, error, ctx)
        };
        runtime_controller::new(self.resource.clone(), Config::default())
        .run(reconcile, error_policy, self.context.clone())
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("reconciled {:?}", o),
                Err(e) => warn!("reconcile failed: {:?}", e),
            }
        }).await;
        Ok(())
    }
}
