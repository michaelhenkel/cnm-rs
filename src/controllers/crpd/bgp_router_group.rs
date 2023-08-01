use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::controllers::crpd::junos::routing_instance;
use crate::controllers::{controllers, bgp_router};
use crate::resources::bgp_router_group::{
    BgpRouterGroup,
    BgpRouterGroupStatus,
    BgpRouterReference,
};
use crate::resources::routing_instance::RoutingInstance;
use crate::resources::bgp_router::{
    BgpRouter,
    BgpRouterParent,
};
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
use ring::digest::{Context as ring_context, SHA512};
use data_encoding::HEXLOWER;
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
                        
                        if bgp_router_group.spec.discover{
                            return handle_bgp_router(&mut bgp_router_group, ctx).await;
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

pub async fn handle_bgp_router(bgp_router_group: &mut BgpRouterGroup, ctx: Arc<Context>) -> Result<Action, ReconcileError>{
    if let Some(routing_instance_ref) = &bgp_router_group.spec.bgp_router_template.routing_instance_parent{
        let routing_instance = match controllers::get::<RoutingInstance>(
            bgp_router_group.meta().namespace.as_ref().unwrap(),
            routing_instance_ref.name.as_ref().unwrap(), 
            ctx.client.clone()).await{
                Ok(res) => {
                    match res{
                        Some((routing_instance, _api)) => {
                            routing_instance
                        },
                        None => return Ok(Action::await_change())
                    }
                },
                Err(e) => return Err(e)
            };
    }
    
    let crpd = match controllers::get::<Crpd>(
        bgp_router_group.meta().namespace.as_ref().unwrap(),
        bgp_router_group.spec.bgp_router_template.instance_parent.as_ref().unwrap().name.as_ref().unwrap(), 
        ctx.client.clone()).await{
            Ok(res) => {
                match res{
                    Some((crpd, _api)) => {
                        crpd
                    },
                    None => return Ok(Action::await_change())
                }
            },
            Err(e) => return Err(e)
        };


    
    if let Some(status) = &crpd.status{
        if let Some(instances) = &status.instances{
            let mut bgp_router_list = Vec::new();
            let mut bgp_router_references = Vec::new();
            for (instance_name, instance) in instances{
                let mut bgp_router_spec = bgp_router_group.spec.bgp_router_template.clone();
                if bgp_router_spec.v4_address.is_none(){
                    if let Some(interface) = &bgp_router_spec.interface{
                        if let Some(interface_config) = instance.interfaces.get(interface){
                            bgp_router_spec.v4_address = interface_config.v4_address.clone();
                        }
                    }
                }
                if bgp_router_spec.v6_address.is_none(){
                    if let Some(interface) = &bgp_router_spec.interface{
                        if let Some(interface_config) = instance.interfaces.get(interface){
                            bgp_router_spec.v6_address = interface_config.v6_address.clone();
                        }
                    }
                }
                if bgp_router_spec.router_id.is_none(){
                    if let Some(interface) = &bgp_router_spec.interface{
                        if let Some(interface_config) = instance.interfaces.get(interface){
                            bgp_router_spec.router_id = interface_config.v4_address.clone();
                        }
                    }
                }
                let mut bgp_router_labels = bgp_router_group.meta().labels.clone();
                bgp_router_labels.as_mut().unwrap().insert("cnm.juniper.net/bgpRouterGroup".to_string(), bgp_router_group.meta().name.as_ref().unwrap().clone());
                if bgp_router_spec.managed{
                    bgp_router_labels.as_mut().unwrap().insert("cnm.juniper.net/bgpRouterManaged".to_string(), "true".to_string());
                }
                let name_namespace = format!("{}{}", instance_name.clone(), crpd.meta().namespace.as_ref().unwrap().clone().to_string());
                let bgp_router_name = format!("{}-{}-{}", instance_name.clone(), bgp_router_group.meta().name.as_ref().unwrap().clone(), generate_hash(&name_namespace));
                let bgp_router = BgpRouter{
                    metadata: meta_v1::ObjectMeta {
                        name: Some(bgp_router_name),
                        namespace: Some(bgp_router_group.meta().namespace.as_ref().unwrap().clone()),
                        labels: bgp_router_labels,
                        owner_references: Some(vec![
                            meta_v1::OwnerReference{
                                api_version: "v1".to_string(),
                                kind: "Pod".to_string(),
                                name: instance_name.clone(),
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
                                local_v4_address: bgp_router.spec.v4_address.clone(),
                                local_v6_address: bgp_router.spec.v6_address.clone(),
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
            match controllers::update_status(bgp_router_group.clone(), ctx.client.clone()).await {
                Ok(_) => {

                },
                Err(e) => {
                    return Err(e);
                }
            }
        }
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
        config.label_selector = Some("cnm.juniper.net/bgpRouterType=Crpd".to_string());
        runtime_controller::new(self.resource.clone(), config.clone())
            .watches(
                Api::<Crpd>::all(self.context.client.clone()),
                Config::default(),
                |crpd| {
                    info!("crpd event in bgp_router_group controller:");
                    let mut object_list = Vec::new();
                    match crpd.status{
                        Some(status) => {
                            match status.bgp_router_group_references{
                                Some(bgp_router_group_refs) => {
                                    
                                    for bgp_router_group_ref in &bgp_router_group_refs{
                                        let object = ObjectRef::<BgpRouterGroup>::new(
                                            bgp_router_group_ref.name.as_ref().unwrap().clone().as_str())
                                            .within(bgp_router_group_ref.namespace.as_ref().unwrap());
                                        object_list.push(object);
                                    }
                                },
                                None => {}
                            }
                        },
                        None => {}
                    }
                    object_list.into_iter()
                }
            )
            .watches(
                Api::<BgpRouter>::all(self.context.client.clone()),
                Config::default(),
                |bgp_router| {
                    info!("bgp_router event in bgp_router_group controller:");
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
            .watches(
                Api::<RoutingInstance>::all(self.context.client.clone()),
                Config::default(),
                |routing_instance| {
                    info!("routing_instance event in bgp_router_group controller:");
                    let mut object_ref_list = Vec::new();
                    if let Some(status) = routing_instance.status{
                        if let Some(bgp_router_group_references) = status.bgp_router_group_references{
                            for bgp_router_group_reference in bgp_router_group_references{
                                object_ref_list.push(ObjectRef::<BgpRouterGroup>::new(
                                    bgp_router_group_reference.name.as_ref().unwrap())
                                    .within(bgp_router_group_reference.namespace.as_ref().unwrap()))
                            }
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

fn generate_hash(input: &str) -> String {
    let mut context = ring_context::new(&SHA512);
    context.update(input.as_bytes());
    let digest = context.finish();
    let hex = HEXLOWER.encode(digest.as_ref());
    let hash = hex[..8].to_string();
    hash
}
