use crate::controllers::controllers::{
    Controller, Context, ReconcileError, self
};
use crate::resources::routing_instance_group::RoutingInstanceGroup;
use crate::resources::vrrp_group::VrrpGroup;
use crate::resources::{
    crpd::crpd::{Crpd, CrpdStatus},
    crpd::crpd_group::{CrpdGroup, CrpdGroupStatus},
    bgp_router_group::BgpRouterGroup,
    interface_group::InterfaceGroup,
    resources
};
use async_trait::async_trait;
use futures::StreamExt;
use kube::{
    Resource,
    api::Api,
    runtime::{
        controller::{Action, Controller as runtime_controller},
        watcher::Config,
        reflector::ObjectRef
    },
};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;
use k8s_openapi::{
    api::{
        apps::v1 as apps_v1,
        core::v1 as core_v1,
        rbac::v1 as rbac_v1,
    },
    apimachinery::pkg::apis::meta::v1 as meta_v1,
};


pub struct CrpdController{
    context: Arc<Context>,
    resource: Api<Crpd>,
}

impl CrpdController{
    pub fn new(context: Arc<Context>) -> Self{
        let resource = Api::all(context.client.clone());
        let context = context.clone();
        CrpdController{context, resource}
    }
    async fn reconcile(g: Arc<Crpd>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        let name = g.meta().name.as_ref().unwrap();
        let namespace = g.meta().namespace.as_ref().unwrap();
        match controllers::get::<Crpd>(g.meta().namespace.as_ref().unwrap(),
            g.meta().name.as_ref().unwrap(),
            ctx.client.clone())
            .await{
            Ok(res) => {
                match res{
                    Some((mut crpd, _crpd_api)) => {
                        Ok(Action::await_change())
                    },
                    None => {
                        info!("crpd does not exist");
                        Ok(Action::await_change())
                    },
                }
            },
            Err(e) => {
                return Err(e);
            },
        }
    }
    fn error_policy(_g: Arc<Crpd>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

#[async_trait]
impl Controller for CrpdController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<Crpd>, ctx: Arc<Context>| {
            async move { CrpdController::reconcile(g, ctx).await }
        };


        let error_policy = |g: Arc<Crpd>, error: &ReconcileError, ctx: Arc<Context>| {
            CrpdController::error_policy(g, error, ctx)
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