use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::controllers::{controllers, bgp_router};
use crate::resources::bgp_router_group::BgpRouterGroup;
use crate::resources::bgp_router_group::BgpRouterGroupStatus;
use crate::resources::bgp_router_group::BgpRouterReference;
use crate::resources::bgp_router::BgpRouter;
use crate::resources::crpd::crpd::Crpd;
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
        info!("reconciling BgpRouterGroup {:?}", g.meta());
        match controllers::get::<BgpRouterGroup>(
            g.meta().namespace.as_ref().unwrap().clone(),
            g.meta().name.as_ref().unwrap().clone(),
            ctx.client.clone())
            .await{
            Ok(res) => {
                match res{
                    Some((mut bgp_router_group, _api)) => {
                        if bgp_router_group.spec.discover{
                            match controllers::get::<Crpd>(g.meta().namespace.as_ref().unwrap().clone(),
                            g.meta().name.as_ref().unwrap().clone(),
                            ctx.client.clone())
                            .await{
                                Ok(res) => {
                                    match res {
                                        Some((crpd, _crpd_api)) => {
                                            if let Some(status) = &crpd.status{
                                                if let Some(instances) = &status.instances{
                                                    let mut bgp_router_list = Vec::new();
                                                    let mut bgp_router_references = Vec::new();
                                                    for instance in instances{
                                                        let mut bgp_router_spec = bgp_router_group.spec.bgp_router_template.clone();
                                                        bgp_router_spec.address = Some(instance.address.clone());
                                                        bgp_router_spec.router_id = Some(instance.address.clone());
                                                        let mut bgp_router_labels = bgp_router_group.meta().labels.clone();
                                                        bgp_router_labels.as_mut().unwrap().insert("cnm.juniper.net/bgpRouterGroup".to_string(), bgp_router_group.meta().name.as_ref().unwrap().clone());
                                                        if bgp_router_spec.managed{
                                                            bgp_router_labels.as_mut().unwrap().insert("cnm.juniper.net/bgpRouterManaged".to_string(), "true".to_string());
                                                        }
                                                        let bgp_router = BgpRouter{
                                                            metadata: meta_v1::ObjectMeta {
                                                                name: Some(instance.name.clone()),
                                                                namespace: Some(g.meta().namespace.as_ref().unwrap().clone()),
                                                                labels: bgp_router_labels,
                                                                owner_references: Some(vec![
                                                                    meta_v1::OwnerReference{
                                                                        api_version: "v1".to_string(),
                                                                        kind: "Pod".to_string(),
                                                                        name: instance.name.clone(),
                                                                        uid:  instance.uuid.clone(),
                                                                        ..Default::default()
                                                                    },
                                                                ]),
                                                                ..Default::default()
                                                            },
                                                            spec: bgp_router_spec,
                                                            status: None,
                                                        };
                                                        match controllers::create_or_update(bgp_router, ctx.client.clone()).await{
                                                            Ok(bgp_router) => {
                                                                if let Some(bgp_router) = bgp_router{
                                                                    let bgp_router_reference = BgpRouterReference { 
                                                                        bgp_router_reference:  core_v1::ObjectReference{
                                                                            api_version: Some("cnm.juniper.net/v1".to_string()),
                                                                            kind: Some("BgpRouter".to_string()),
                                                                            name: Some(bgp_router.meta().name.as_ref().unwrap().clone()),
                                                                            uid: Some(bgp_router.meta().uid.as_ref().unwrap().clone()),
                                                                            ..Default::default()
                                                                        },
                                                                        local_address: bgp_router.spec.address.clone().unwrap(),
                                                                    };
                                                                    bgp_router_references.push(bgp_router_reference);
                                                                    bgp_router_list.push(bgp_router);
                                                                }
                                                            },
                                                            Err(e) => {
                                                                return Err(e);
                                                            }
                                                        }
                                                    }
                                                    if bgp_router_group.status.is_some(){
                                                        bgp_router_group.status.as_mut().unwrap().bgp_router_references = bgp_router_references.clone();
                                                    } else {
                                                        bgp_router_group.status = Some(BgpRouterGroupStatus{
                                                            bgp_router_references: bgp_router_references.clone(),
                                                        });
                                                    }  
                                                    match controllers::update_status(bgp_router_group, ctx.client.clone()).await {
                                                        Ok(_) => {

                                                        },
                                                        Err(e) => {
                                                            return Err(e);
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                        None => {}
                                    }
                                },
                                Err(e) => {
                                    return Err(e);
                                }
                            }
                            Ok(Action::await_change())
                        } else {
                            Ok(Action::await_change())
                        }
                    },
                    None => {
                        info!("crpd bgp_router_group does not exist");
                        Ok(Action::await_change())
                    }
                    
                }
            },
            Err(e) => {
                return Err(e);
            },
        }
    }
    fn error_policy(g: Arc<BgpRouterGroup>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
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
        config.label_selector = Some("cnm.juniper.net/bgpRouterType=Crpd".to_string());
        runtime_controller::new(self.resource.clone(), config.clone())
            .watches(
                Api::<Crpd>::all(self.context.client.clone()),
                Config::default(),
                |crpd| {
                    info!("crpd event in bgp_router_group controller:");
                    Some(ObjectRef::<BgpRouterGroup>::new(
                        crpd.meta().name.as_ref().unwrap())
                        .within(crpd.meta().namespace.as_ref().unwrap()))
                }
            )
            .watches(
                Api::<BgpRouter>::all(self.context.client.clone()),
                Config::default(),
                |bgp_router| {
                    info!("crpd event in bgp_router_group controller:");
                    match &bgp_router.meta().labels{
                        Some(labels) => {
                            match labels.get("cnm.juniper.net/bgpRouterGroup"){
                                Some(bgp_router_group_name) => {
                                    Some(ObjectRef::<BgpRouterGroup>::new(
                                        bgp_router_group_name)
                                        .within(bgp_router.meta().namespace.as_ref().unwrap()))
                                },
                                None => {
                                    None
                                }
                            }
                        },
                        None => {
                            None
                        }
                    }
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
