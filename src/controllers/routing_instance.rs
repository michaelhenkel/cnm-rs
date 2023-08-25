use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::controllers::controllers;

use crate::resources::routing_instance::{
    RoutingInstance
};
use crate::resources::routing_instance_group::RoutingInstanceGroup;
use kube::Resource;
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

use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;


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
            g.meta().namespace.as_ref().unwrap(),
            g.meta().name.as_ref().unwrap(),
            ctx.client.clone())
            .await{
            Ok(res) => {
                match res{
                    Some((_routing_instance, _api)) => {},
                    None => {}
                }
            },
            Err(e) => return Err(e)
        }
        Ok(Action::await_change())
    }
    fn error_policy(_g: Arc<RoutingInstance>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
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
            .watches(
                Api::<RoutingInstanceGroup>::all(self.context.client.clone()),
                Config::default(),
                |group| {
                    info!("routing_instance_group event in routing_instance controller:");
                    let mut object_list = Vec::new();
                    let namespace = group.meta().namespace.as_ref().unwrap();
                    if let Some(status) = &group.status{
                        if let Some(refs) = &status.routing_instance_references{
                            object_list = refs.iter().map(|obj_ref|{
                                ObjectRef::<RoutingInstance>::new(
                                    obj_ref.name.as_ref().unwrap().clone().as_str())
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
