use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::resources::bgp_router::{BgpRouter, BgpRouterStatus, BgpPeeringReference, BgpSessionAttributes};
use crate::resources::bgp_router_group::BgpRouterGroup;
use crate::controllers::controllers;
use async_trait::async_trait;
use futures::StreamExt;
use kube::{
    Resource,
    api::Api,
    client::Client,
    runtime::{
        controller::{Action, Controller as runtime_controller},
        watcher::Config,
        reflector::ObjectRef,
    },
};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;

pub struct BgpRouterController{
    context: Arc<Context>,
    resource: Api<BgpRouter>,
}

impl BgpRouterController{
    pub fn new(context: Arc<Context>) -> Self{
        let resource = Api::all(context.client.clone());
        let context = context.clone();
        BgpRouterController{context, resource}
    }
    async fn reconcile(g: Arc<BgpRouter>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        info!("reconciling BgpRouter {:?}", g.meta().name.as_ref().unwrap().clone());
        match controllers::get::<BgpRouter>(
            g.meta().namespace.as_ref().unwrap(),
            g.meta().name.as_ref().unwrap(),
            ctx.client.clone())
            .await{
            Ok(res) => {
                match res{
                    Some((mut bgp_router, _bgp_router_api)) => {
                        if let Some(labels) = &bgp_router.meta().labels{
                            if let Some(bgp_router_group_name) = labels.get("cnm.juniper.net/bgpRouterGroup"){
                                if let Ok(res) = controllers::get::<BgpRouterGroup>(bgp_router.meta().namespace.as_ref().unwrap(), bgp_router_group_name, ctx.client.clone()).await{
                                    if let Some((bgp_router_group, _)) = res{
                                        if let Some(bgp_router_group_status) = bgp_router_group.status{
                                            let mut bgp_peering_references = Vec::new();
                                            for bgp_router_reference in &bgp_router_group_status.bgp_router_references{
                                                if bgp_router_reference.bgp_router_reference.name.as_ref().unwrap().clone() != bgp_router.meta().name.as_ref().unwrap().clone(){
                                                    let bgp_peering_reference = BgpPeeringReference{
                                                        peer_reference: bgp_router_reference.bgp_router_reference.clone(),
                                                        bgp_router_group: Some(bgp_router_group_name.to_string()),
                                                        session_attributes: BgpSessionAttributes{
                                                            local_v4_address: bgp_router.spec.v4_address.clone(),
                                                            peer_v4_address: bgp_router_reference.local_v4_address.clone(),
                                                            local_v6_address: bgp_router.spec.v6_address.clone(),
                                                            peer_v6_address: bgp_router_reference.local_v6_address.clone(),
                                                            local_as: bgp_router.spec.autonomous_system_number,
                                                            peer_as: bgp_router.spec.autonomous_system_number,
                                                            address_families: bgp_router.spec.address_families.clone(),
                                                        }
                                                    };
                                                    bgp_peering_references.push(bgp_peering_reference);
                                                }
                                            }

                                            if bgp_router.status.is_some(){
                                                bgp_router.status.as_mut().unwrap().bgp_peer_references = Some(bgp_peering_references);
                                            } else {
                                                bgp_router.status = Some(BgpRouterStatus{
                                                    bgp_peer_references: Some(bgp_peering_references),
                                                });
                                            }  

                                            if let Err(e) = controllers::update_status::<BgpRouter>(bgp_router.clone(), ctx.client.clone()).await{
                                                return Err(e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    None => {}
                }
            }
            Err(e) => {
               return Err(e)
            }
        }
        Ok(Action::await_change())
    }
    fn error_policy(g: Arc<BgpRouter>, error: &ReconcileError, ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

#[async_trait]
impl Controller for BgpRouterController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<BgpRouter>, ctx: Arc<Context>| {
            async move { BgpRouterController::reconcile(g, ctx).await }
        };
        let error_policy = |g: Arc<BgpRouter>, error: &ReconcileError, ctx: Arc<Context>| {
            BgpRouterController::error_policy(g, error, ctx)
        };
        runtime_controller::new(self.resource.clone(), Config::default())
            .watches(
                Api::<BgpRouterGroup>::all(self.context.client.clone()),
                Config::default(),
                |bgp_router_group| {
                    info!("crpd event in bgp_router_group controller:");
                    let mut object_ref_list = Vec::new();
                    if let Some(status) = &bgp_router_group.status{
                        for bgp_router_reference in &status.bgp_router_references{
                            let object_ref = ObjectRef::<BgpRouter>::new(
                                bgp_router_reference.bgp_router_reference.name.as_ref().unwrap())
                                .within(bgp_router_group.meta().namespace.as_ref().unwrap().clone().as_str());
                            object_ref_list.push(object_ref);
                        }
                    }
                    object_ref_list.into_iter()
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

fn test(obj: &BgpRouterGroup) -> Option<u64>{
    Some(0)
}
