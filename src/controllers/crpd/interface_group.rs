use crate::controllers::controllers::{Controller, Context, ReconcileError};

use crate::controllers::controllers;
use crate::resources::interface::Interface;
use crate::resources::crpd::crpd::Crpd;
use crate::resources::crpd::crpd;
use crate::resources::{vrrp, resources};
use crate::resources::vrrp_group;

use crate::resources::interface_group::{
    InterfaceGroup,
    InterfaceSelector,
    InterfaceGroupStatus
};
use crate::resources::interface;
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
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;
use k8s_openapi::api::core::v1 as core_v1;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta_v1;

pub struct InterfaceGroupController{
    context: Arc<Context>,
    resource: Api<InterfaceGroup>,
}

impl InterfaceGroupController{
    pub fn new(context: Arc<Context>) -> Self{
        let resource = Api::all(context.client.clone());
        
        InterfaceGroupController{context, resource}
    }
    async fn reconcile(g: Arc<InterfaceGroup>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        let name = g.meta().name.as_ref().unwrap();
        let namespace = g.meta().namespace.as_ref().unwrap();
        let mut interface_group = match controllers::get::<InterfaceGroup>(namespace,name,ctx.client.clone()).await{
            Ok(res) => {
                match res{
                    Some((routing_instance_group, _api)) => {
                        routing_instance_group
                    },
                    None => return Ok(Action::await_change())
                }
            },
            Err(e) => return Err(e)
        };
        if let Some(instance_parent) = &g.spec.interface_template.instance_parent{
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
                        for (instance_name, instance) in instance_map{
                            let mut instance_interface_map = BTreeMap::new();
                            match &interface_group.spec.interface_selector{
                                InterfaceSelector::List(interface_list) => {
                                    for intf in interface_list{
                                        if let Some(interface) = instance.interfaces.get(intf){
                                            instance_interface_map.insert(intf.clone(), interface.clone());
                                        }
                                    }
                                },
                                InterfaceSelector::All(_) => {
                                    instance_interface_map = instance.interfaces.clone();
                                }
                            };
                            for (instance_interface_name, instance_interface) in &instance_interface_map{
                                let mut interface_spec = interface_group.spec.interface_template.clone();
                                let mut interface_families = Vec::new();
                                if let Some(v4) = &instance_interface.v4_address{
                                    interface_families.push(interface::InterfaceFamily::Inet(
                                        interface::InterfaceInet{
                                            address: v4.clone(),
                                        }
                                    ));
                                }
                                if let Some(v6) = &instance_interface.v6_address{
                                    interface_families.push(interface::InterfaceFamily::Inet6(
                                        interface::InterfaceInet6{
                                            address: v6.clone(),
                                        }
                                    ));
                                }
                                if interface_families.len() > 0 {
                                    interface_spec.families = Some(interface_families);
                                }
                                interface_spec.name = Some(instance_interface_name.clone());
                                interface_spec.mac = Some(instance_interface.mac.clone());
                                let interface_name = format!("{}-{}", instance_name, instance_interface_name);
                                let mut interface = Interface::new(interface_name.as_str(), interface_spec);
                                interface.metadata.namespace = Some(namespace.clone());
                                interface.metadata.labels = Some(BTreeMap::from([
                                    ("cnm.juniper.net/instanceSelector".to_string(), crpd.meta().name.as_ref().unwrap().clone()),
                                    ("cnm.juniper.net/interfaceGroup".to_string(), name.clone()),
                                    ("cnm.juniper.net/instanceType".to_string(), resources::InstanceType::Crpd.to_string()),
                                ]));
                                match controllers::get::<core_v1::Pod>(namespace,instance_name,ctx.client.clone()).await{
                                    Ok(res) => {
                                        if let Some((pod, _)) = res {
                                            interface.metadata.owner_references = Some(vec![meta_v1::OwnerReference{
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
        
                                if let Err(e) = controllers::create_or_update(interface, ctx.client.clone()).await{
                                    return Err(e);
                                }
                                
                            }
                        }
                    }
                }
            }
        }
        match controllers::list::<Interface>(namespace, ctx.client.clone(), Some(BTreeMap::from([
            ("cnm.juniper.net/bgpRouterGroup".to_string(), name.clone())
        ]))).await{
            Ok(res) => {
                if let Some((child_list,_)) = res {
                    let ref_list: Vec<core_v1::LocalObjectReference> = child_list.iter().map(|obj|{
                        core_v1::LocalObjectReference{
                            name: Some(obj.meta().name.as_ref().unwrap().clone()),
                        }
                    }).collect();
                    if ref_list.len() > 0 {
                        if let Some(status) = interface_group.status.as_mut(){
                            status.interface_references = Some(ref_list);
                        } else {
                            interface_group.status = Some(InterfaceGroupStatus{
                                interface_references: Some(ref_list),
                            })
                        }
                        if let Err(e) = controllers::update_status(interface_group, ctx.client.clone()).await{
                            return Err(e);
                        }
                    }
                } 
            },
            Err(e) => return Err(e)
        }
        return Ok(Action::await_change())
    }
    fn error_policy(_g: Arc<InterfaceGroup>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

#[async_trait]
impl Controller for InterfaceGroupController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<InterfaceGroup>, ctx: Arc<Context>| {
            async move { InterfaceGroupController::reconcile(g, ctx).await }
        };
        let error_policy = |g: Arc<InterfaceGroup>, error: &ReconcileError, ctx: Arc<Context>| {
            InterfaceGroupController::error_policy(g, error, ctx)
        };
        runtime_controller::new(self.resource.clone(), Config::default())
            .watches(
                Api::<Crpd>::all(self.context.client.clone()),
                Config::default(),
                |crpd| {
                    info!("crpd event in routing_instance_group controller:");
                    let mut object_list = Vec::new();
                    if let Some(status) = &crpd.status{
                        if let Some(refs) = &status.interface_group_references{
                            object_list = refs.iter().map(|obj_ref|{
                                ObjectRef::<InterfaceGroup>::new(
                                    obj_ref.name.as_ref().unwrap().clone().as_str())
                                    .within(crpd.meta().namespace.as_ref().unwrap())
                            }).collect();
                        }
                    }
                    object_list.into_iter()
                }
            )
            .watches(
                Api::<Interface>::all(self.context.client.clone()),
                Config::default(),
                |obj| {
                    info!("bgp_router event in bgp_router_group controller:");
                    if let Some(labels) = &obj.meta().labels{
                        if let Some(parent_group) = labels.get("cnm.juniper.net/interfaceGroup"){
                            return Some(ObjectRef::<InterfaceGroup>::new(parent_group)
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
