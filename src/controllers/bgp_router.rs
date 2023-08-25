use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::resources::bgp_router::{BgpRouter};
use crate::resources::bgp_router_group::BgpRouterGroup;
use crate::controllers::controllers;
use async_trait::async_trait;
use futures::StreamExt;
use kube::{
    Resource,
    api::Api,
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
                    Some((_bgp_router, _bgp_router_api)) => {},
                    None => {}
                }
            }
            Err(e) => {
               return Err(e)
            }
        }
        Ok(Action::await_change())
    }
    fn error_policy(_g: Arc<BgpRouter>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
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
                |group| {
                    info!("bgp_router_group event in bgp_router controller:");
                    let mut object_list = Vec::new();
                    let namespace = group.meta().namespace.as_ref().unwrap();
                    if let Some(status) = &group.status{
                        if let Some(obj_refs) = &status.bgp_router_references{
                            object_list = obj_refs.iter().map(|obj_ref|{
                                ObjectRef::<BgpRouter>::new(
                                    obj_ref.bgp_router_reference.name.as_ref().unwrap().clone().as_str())
                                    .within(namespace)
                            }).collect();
                        }
                    }
                    object_list.into_iter()
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