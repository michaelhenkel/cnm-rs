use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::controllers::crpd::junos::bgp;
use crate::controllers::{controllers, bgp_router};
use crate::resources::bgp_router_group::BgpRouterGroup;
use crate::resources::routing_instance::{
    RoutingInstance,
    RoutingInstanceStatus
};
use kube::Resource;
use async_trait::async_trait;
use futures::StreamExt;
use kube::{
    api::Api,
    client::Client,
    runtime::{
        controller::{Action, Controller as runtime_controller},
        watcher::Config,
    },
};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;
use k8s_openapi::api::core::v1 as core_v1;

pub struct RoutingInstanceController{
    context: Arc<Context>,
    resource: Api<RoutingInstance>,
}

impl RoutingInstanceController{
    pub fn new(context: Arc<Context>) -> Self{
        let resource = Api::all(context.client.clone());
        
        RoutingInstanceController{context, resource}
    }
    async fn reconcile(g: Arc<RoutingInstance>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        info!("reconciling RoutingInstance {:?}", g.meta().name.as_ref().unwrap().clone());
        
        match controllers::get::<RoutingInstance>(
            g.meta().namespace.as_ref().unwrap().clone(),
            g.meta().name.as_ref().unwrap().clone(),
            ctx.client.clone())
            .await{
            Ok(res) => {
                match res{
                    Some((mut routing_instance, _api)) => {
                        let update_status = match controllers::list::<BgpRouterGroup>(
                            routing_instance.meta().namespace.as_ref().unwrap().clone(),
                            ctx.client.clone(),
                            Some(BTreeMap::from([("cnm.juniper.net/routingInstance".to_string(), routing_instance.meta().name.as_ref().unwrap().clone())]))
                        ).await{
                            Ok(res) => {
                                match res{
                                    Some((bgp_router_groups, _api)) => {
                                        let mut bgp_router_group_refs = Vec::new();
                                        for bgp_router_group in bgp_router_groups{
                                            let bgp_router_group_ref = core_v1::ObjectReference{
                                                api_version: Some("cnm.juniper.net/v1".to_string()),
                                                kind: Some("BgpRouterGroup".to_string()),
                                                name: Some(bgp_router_group.meta().name.as_ref().unwrap().clone()),
                                                namespace: Some(bgp_router_group.meta().namespace.as_ref().unwrap().clone()),
                                                ..Default::default()
                                            };
                                            bgp_router_group_refs.push(bgp_router_group_ref);
                                        }
                                        if let Some(status) = routing_instance.status.as_mut(){
                                            status.bgp_router_group_references = Some(bgp_router_group_refs);
                                        } else {
                                            let status = RoutingInstanceStatus{
                                                bgp_router_group_references: Some(bgp_router_group_refs),
                                            };
                                            routing_instance.status = Some(status);
                                        }
                                        true
                                    },
                                    None => false
                                }
                            },
                            Err(e) => return Err(e)
                        };
                        if update_status{
                            match controllers::update_status(routing_instance, ctx.client.clone()).await{
                                Ok(_res) => {},
                                Err(e) => return Err(e)
                            }
                        }
                    }
                    None => {}
                }
            },
            Err(e) => return Err(e)
            
        }
    
        Ok(Action::await_change())
    }
    fn error_policy(g: Arc<RoutingInstance>, error: &ReconcileError, ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

#[async_trait]
impl Controller for RoutingInstanceController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<RoutingInstance>, ctx: Arc<Context>| {
            async move { RoutingInstanceController::reconcile(g, ctx).await }
        };
        let error_policy = |g: Arc<RoutingInstance>, error: &ReconcileError, ctx: Arc<Context>| {
            RoutingInstanceController::error_policy(g, error, ctx)
        };
        runtime_controller::new(self.resource.clone(), Config::default())
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
