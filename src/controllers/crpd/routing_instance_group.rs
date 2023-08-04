use crate::controllers::controllers::{Controller, Context, ReconcileError};

use crate::controllers::controllers;

use crate::resources;
use crate::resources::bgp_router::BgpRouter;
use crate::resources::bgp_router_group::BgpRouterGroup;
use crate::resources::routing_instance_group::RoutingInstanceGroupStatus;
use crate::resources::routing_instance::{RoutingInstance, RoutingInstanceSpec};
use crate::resources::crpd::crpd::Crpd;
use crate::resources::routing_instance_group::RoutingInstanceGroup;
use async_trait::async_trait;
use futures::StreamExt;
use kube::{
    api::Api,
    runtime::{
        controller::{Action, Controller as runtime_controller},
        watcher::Config,
    },
};
use ring::digest::{Context as ring_context, SHA512};
use data_encoding::HEXLOWER;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;
use kube::runtime::reflector::ObjectRef;
use kube::Resource;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta_v1;
use k8s_openapi::api::core::v1 as core_v1;


pub struct RoutingInstanceGroupController{
    context: Arc<Context>,
    resource: Api<RoutingInstanceGroup>,
}

impl RoutingInstanceGroupController{
    pub fn new(context: Arc<Context>) -> Self{
        let resource = Api::all(context.client.clone());
        let context = context.clone();
        RoutingInstanceGroupController{context, resource}
    }
    async fn reconcile(g: Arc<RoutingInstanceGroup>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        info!("reconciling RoutingInstanceGroup {:?}", g.meta().name.as_ref().unwrap().clone());
        match controllers::get::<RoutingInstanceGroup>(
            g.meta().namespace.as_ref().unwrap(),
            g.meta().name.as_ref().unwrap(),
            ctx.client.clone())
            .await{
            Ok(res) => {
                match res{
                    Some((mut routing_instance_group, _api)) => {
                        return handle_routing_instance_group(&mut routing_instance_group, ctx).await;
                    },
                    None => Ok(Action::await_change())
                }
            },
            Err(e) => return Err(e)
        }
    }
    fn error_policy(_g: Arc<RoutingInstanceGroup>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

pub async fn handle_routing_instance_group(routing_instance_group: &mut RoutingInstanceGroup, ctx: Arc<Context>) -> Result<Action, ReconcileError>{
    let name = routing_instance_group.meta().name.as_ref().unwrap();
    let namespace = routing_instance_group.meta().namespace.as_ref().unwrap();
    if let Some(instance_parent) = &routing_instance_group.spec.routing_instance_template.instance_parent{
        if let Some(instance_parent_name) = &instance_parent.reference.name{
            let crpd = match controllers::get::<Crpd>(namespace,instance_parent_name,ctx.client.clone()).await{
                Ok(res) => {
                    match res{
                        Some((crpd, _api)) => crpd,
                        None => return Ok(Action::await_change())
                    }
                },
                Err(e) => return Err(e)
            };
            if let Some(status) = &crpd.status{
                if let Some(instance_map) = &status.instances{
                    for (instance_name, _instance) in instance_map{
                        let mut routing_instance = RoutingInstance::new(instance_name, routing_instance_group.spec.routing_instance_template.clone());
                        match controllers::get::<core_v1::Pod>(namespace,instance_name,ctx.client.clone()).await{
                            Ok(res) => {
                                if let Some((pod, _)) = res {
                                    routing_instance.metadata.owner_references = Some(vec![meta_v1::OwnerReference{
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
                        routing_instance.metadata.namespace = Some(namespace.clone());
                        routing_instance.metadata.labels = Some(BTreeMap::from([
                            ("cnm.juniper.net/instanceSelector".to_string(), crpd.meta().name.as_ref().unwrap().clone()),
                            ("cnm.juniper.net/routingInstanceGroup".to_string(), name.clone()),
                            ("cnm.juniper.net/instanceType".to_string(), resources::resources::InstanceType::Crpd.to_string()),
                        ]));
                        if let Err(e) = controllers::create_or_update(routing_instance, ctx.client.clone()).await{
                            return Err(e);
                        }
                    }
                }
            }
        }
    }
    match controllers::list::<RoutingInstance>(namespace, ctx.client.clone(), Some(BTreeMap::from([
        ("cnm.juniper.net/routingInstanceGroup".to_string(), name.clone())
    ]))).await{
        Ok(res) => {
            if let Some((child_list,_)) = res {
                let ref_list: Vec<core_v1::LocalObjectReference> = child_list.iter().map(|obj|{
                    core_v1::LocalObjectReference{
                        name: Some(obj.meta().name.as_ref().unwrap().clone())
                    }
                }).collect();
                if ref_list.len() > 0 {
                    if let Some(status) = routing_instance_group.status.as_mut(){
                        status.routing_instance_references = Some(ref_list);
                    } else {
                        routing_instance_group.status = Some(RoutingInstanceGroupStatus{
                            routing_instance_references: Some(ref_list),
                            bgp_router_group_references: None,
                        })
                    }
                    if let Err(e) = controllers::update_status(routing_instance_group.clone(), ctx.client.clone()).await{
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
impl Controller for RoutingInstanceGroupController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<RoutingInstanceGroup>, ctx: Arc<Context>| {
            async move { RoutingInstanceGroupController::reconcile(g, ctx).await }
        };
        let error_policy = |g: Arc<RoutingInstanceGroup>, error: &ReconcileError, ctx: Arc<Context>| {
            RoutingInstanceGroupController::error_policy(g, error, ctx)
        };
        let mut config = Config::default();
        config.label_selector = Some("cnm.juniper.net/instanceType=Crpd".to_string());
        runtime_controller::new(self.resource.clone(), config.clone())
            .watches(
                Api::<Crpd>::all(self.context.client.clone()),
                Config::default(),
                |crpd| {
                    info!("crpd event in routing_instance_group controller:");
                    let mut object_list = Vec::new();
                    if let Some(status) = &crpd.status{
                        if let Some(refs) = &status.routing_instance_group_references{
                            object_list = refs.iter().map(|obj_ref|{
                                ObjectRef::<RoutingInstanceGroup>::new(
                                    obj_ref.name.as_ref().unwrap().clone().as_str())
                                    .within(crpd.meta().namespace.as_ref().unwrap())
                            }).collect();
                        }
                    }
                    object_list.into_iter()
                }
            )
            .watches(
                Api::<RoutingInstance>::all(self.context.client.clone()),
                Config::default(),
                |obj| {
                    info!("bgp_router event in bgp_router_group controller:");
                    if let Some(labels) = &obj.meta().labels{
                        if let Some(parent_group) = labels.get("cnm.juniper.net/routingInstanceGroup"){
                            return Some(ObjectRef::<RoutingInstanceGroup>::new(parent_group)
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

fn generate_hash(input: &str) -> String {
    let mut context = ring_context::new(&SHA512);
    context.update(input.as_bytes());
    let digest = context.finish();
    let hex = HEXLOWER.encode(digest.as_ref());
    let hash = hex[..8].to_string();
    hash
}
