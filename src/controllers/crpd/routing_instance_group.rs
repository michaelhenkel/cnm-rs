use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::controllers::controllers;
use crate::resources::routing_instance_group::RoutingInstanceGroupStatus;
use crate::resources::routing_instance::RoutingInstance;
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
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;
use kube::Resource;
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
        if let Some(_instance_parent_name) = &instance_parent.reference.name{

  
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