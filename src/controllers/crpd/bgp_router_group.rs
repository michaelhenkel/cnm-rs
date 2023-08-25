use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::resources::crpd::crpd_group::CrpdGroup;
use crate::resources::resources;
use crate::controllers::controllers;
use crate::resources::bgp_router_group::{
    BgpRouterGroup,
    BgpRouterGroupStatus,
    BgpRouterReference,
};
use crate::resources::bgp_router::BgpRouter;
use crate::resources::crpd::crpd::Crpd;
use async_trait::async_trait;
use futures::StreamExt;
use kube::{
    api::Api,
    runtime::{
        controller::{Action, Controller as runtime_controller},
        watcher::Config,
    },
};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;
use kube::runtime::reflector::ObjectRef;
use kube::Resource;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta_v1;
use k8s_openapi::api::core::v1 as core_v1;

pub struct BgpRouterGroupController{
    context: Arc<Context>,
    resource: Api<BgpRouterGroup>,
}

impl BgpRouterGroupController{
    pub fn new(context: Arc<Context>) -> Self{
        let resource = Api::all(context.client.clone());
        let context = context.clone();
        BgpRouterGroupController{context, resource}
    }
    async fn reconcile(g: Arc<BgpRouterGroup>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        info!("reconciling BgpRouterGroup {:?}", g.meta().name.as_ref().unwrap().clone());
        match controllers::get::<BgpRouterGroup>(
            g.meta().namespace.as_ref().unwrap(),
            g.meta().name.as_ref().unwrap(),
            ctx.client.clone())
            .await{
            Ok(res) => {
                match res{
                    Some((mut bgp_router_group, _api)) => {
                        return handle_bgp_router(&mut bgp_router_group, ctx).await;
                    },
                    None =>  Ok(Action::await_change())
                }
            },
            Err(e) => return Err(e)
        }
    }
    fn error_policy(_g: Arc<BgpRouterGroup>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

pub async fn handle_bgp_router(bgp_router_group: &mut BgpRouterGroup, ctx: Arc<Context>) -> Result<Action, ReconcileError>{
    let name = bgp_router_group.meta().name.as_ref().unwrap();
    let namespace = bgp_router_group.meta().namespace.as_ref().unwrap();
    if let Some(instance_parent) = &bgp_router_group.spec.bgp_router_template.instance_parent{
        if let Some(instance_parent_name) = &instance_parent.reference.name{
            let crpd_group = match controllers::get::<CrpdGroup>(namespace,instance_parent_name,ctx.client.clone()).await{
                Ok(res) => {
                    match res{
                        Some((crpd, _api)) => crpd,
                        None => return Ok(Action::await_change())
                    }
                },
                Err(e) => return Err(e)
            };

            if let Some(status) = &crpd_group.status{
                if let Some(crpd_references) = &status.crpd_references{
                    for crpd_reference in crpd_references{
                        match controllers::get::<Crpd>(namespace, crpd_reference.name.as_ref().unwrap(), ctx.client.clone()).await{
                            Ok(res) => {
                                if let Some((crpd,_)) = res {
                                    let crpd_name = crpd.meta().name.as_ref().unwrap();
                                    let mut bgp_router = BgpRouter::new(crpd_name, bgp_router_group.spec.bgp_router_template.clone());
                                    bgp_router.metadata.namespace = Some(namespace.clone());
                                    bgp_router.metadata.labels = Some(BTreeMap::from([
                                        ("cnm.juniper.net/instanceSelector".to_string(), crpd_name.to_string()),
                                        ("cnm.juniper.net/routingInstanceGroup".to_string(), name.clone()),
                                        ("cnm.juniper.net/instanceType".to_string(), resources::InstanceType::Crpd.to_string()),
                                    ]));
                                    match controllers::get::<core_v1::Pod>(namespace,crpd_name,ctx.client.clone()).await{
                                        Ok(res) => {
                                            if let Some((pod, _)) = res {
                                                bgp_router.metadata.owner_references = Some(vec![meta_v1::OwnerReference{
                                                    api_version: "v1".to_string(),
                                                    block_owner_deletion: Some(false),
                                                    controller: Some(false),
                                                    kind: "Pod".to_string(),
                                                    name: pod.meta().name.as_ref().unwrap().clone(),
                                                    uid: pod.meta().uid.as_ref().unwrap().clone(),
                                                    ..Default::default()
                                                }]);
                                            } else { return Ok(Action::await_change()) }
                                        }
                                        Err(e) => return Err(e)
                                    }
            
                                    if let Err(e) = controllers::create_or_update(bgp_router, ctx.client.clone()).await{
                                        return Err(e);
                                    }
                                }
                            },
                            Err(e) => return Err(e)
                        }
                    }
                }
            }
        }
    }

    match controllers::list::<BgpRouter>(namespace, ctx.client.clone(), Some(BTreeMap::from([
        ("cnm.juniper.net/bgpRouterGroup".to_string(), name.clone())
    ]))).await{
        Ok(res) => {
            if let Some((child_list,_)) = res {
                let ref_list: Vec<BgpRouterReference> = child_list.iter().map(|obj|{
                    BgpRouterReference{
                        bgp_router_reference: core_v1::LocalObjectReference { 
                            name: Some(obj.meta().name.as_ref().unwrap().clone()),
                        },
                        local_v4_address: obj.spec.v4_address.clone(),
                        local_v6_address: obj.spec.v6_address.clone(),
                    }
                }).collect();
                if ref_list.len() > 0 {
                    if let Some(status) = bgp_router_group.status.as_mut(){
                        status.bgp_router_references = Some(ref_list);
                    } else {
                        bgp_router_group.status = Some(BgpRouterGroupStatus{
                            bgp_router_references: Some(ref_list),
                        })
                    }
                    if let Err(e) = controllers::update_status(bgp_router_group.clone(), ctx.client.clone()).await{
                        return Err(e);
                    }
                }
            } 
        },
        Err(e) => return Err(e)
    }
    return Ok(Action::await_change())
}

#[async_trait]
impl Controller for BgpRouterGroupController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<BgpRouterGroup>, ctx: Arc<Context>| {
            async move { BgpRouterGroupController::reconcile(g, ctx).await }
        };
        let error_policy = |g: Arc<BgpRouterGroup>, error: &ReconcileError, ctx: Arc<Context>| {
            BgpRouterGroupController::error_policy(g, error, ctx)
        };
        let mut config = Config::default();
        config.label_selector = Some("cnm.juniper.net/instanceType=Crpd".to_string());
        runtime_controller::new(self.resource.clone(), config.clone())
            .watches(
                Api::<CrpdGroup>::all(self.context.client.clone()),
                Config::default(),
                |crpd| {
                    info!("crpd event in bgp_router_group controller:");
                    let mut object_list = Vec::new();
                    if let Some(status) = &crpd.status{
                        if let Some(refs) = &status.bgp_router_group_references{
                            object_list = refs.iter().map(|obj_ref|{
                                ObjectRef::<BgpRouterGroup>::new(
                                    obj_ref.name.as_ref().unwrap().clone().as_str())
                                    .within(crpd.meta().namespace.as_ref().unwrap())
                            }).collect();
                        }
                    }
                    object_list.into_iter()
                }
            )
            .watches(
                Api::<BgpRouter>::all(self.context.client.clone()),
                Config::default(),
                |obj| {
                    info!("bgp_router event in bgp_router_group controller:");
                    if let Some(labels) = &obj.meta().labels{
                        if let Some(parent_group) = labels.get("cnm.juniper.net/bgpRouterGroup"){
                            return Some(ObjectRef::<BgpRouterGroup>::new(parent_group)
                                .within(obj.meta().namespace.as_ref().unwrap()));
                        }
                    }
                    None
                }
            )
            .run(reconcile, error_policy, self.context.clone())
            .for_each(|res| async move {
                match res {
                    Ok(o) => info!("reconciled {:?}", o),
                    Err(e) => warn!("reconcile failed: {:?}", e),
                }
            })
            .await;
        Ok(())
    }
}