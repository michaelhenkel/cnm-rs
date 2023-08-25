use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::resources::resources::InstanceType;
use std::time::Duration;
use crate::controllers::controllers;
use crate::resources::interface::{Interface, self};
use crate::resources::crpd::{
    crpd::Crpd,
    crpd_group::CrpdGroup,
};
use crate::resources::resources;
use crate::resources::vrrp_group::VrrpGroup;
use crate::resources::interface_group::{
    InterfaceGroup,
    InterfaceGroupStatus
};
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
use std::collections::BTreeMap;
use std::sync::Arc;
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
                    Some((interface_group, api)) => {
                        if interface_group.meta().deletion_timestamp.is_none(){
                            if let Err(e) = controllers::add_finalizer(api, name).await {
                                return Err(e)
                            }
                        } else if interface_group.meta().deletion_timestamp.is_some() {
                            match controllers::list::<Interface>(namespace, ctx.client.clone(), Some(BTreeMap::from([
                                ("cnm.juniper.net/instanceSelector".to_string(), name.clone()),
                                ("cnm.juniper.net/instanceType".to_string(), InstanceType::Crpd.to_string())
                            ]))).await{
                                Ok(res) => {
                                    if let Some((interface_list, _)) = res {
                                        for interface in &interface_list{
                                            if let Err(e) = controllers::delete::<interface::Interface>(namespace.to_string(), interface.meta().name.as_ref().unwrap().clone(), ctx.client.clone()).await{
                                                return Err(e);
                                            } 
                                        }
                                    }
                                },
                                Err(e) => return Err(e)
                            }
                            if let Err(e) = controllers::del_finalizer(api, name).await {
                                return Err(e)
                            }
                            return Ok(Action::await_change())
                        }
                        interface_group
                    },
                    None => return Ok(Action::await_change())
                }
            },
            Err(e) => return Err(e)
        };
        if let Some(instance_parent) = &g.spec.interface_template.instance_parent{
            if let Some(instance_parent_name) = &instance_parent.reference.name{
                let crpd_group = match controllers::get::<CrpdGroup>(namespace,instance_parent_name,ctx.client.clone()).await{
                    Ok(res) => {
                        match res{
                            Some((crpd, _api)) => crpd,
                            None => return Ok(Action::await_change())
                        }
                    },
                    Err(e) => return Err(e)
                };

                if let Some(status) = &crpd_group.status{
                    if let Some(crpd_references) = &status.crpd_references{
                        for crpd_reference in crpd_references{
                            match controllers::get::<Crpd>(namespace, crpd_reference.name.as_ref().unwrap(), ctx.client.clone()).await{
                                Ok(res) => {
                                    if let Some((crpd,_)) = res {
                                        let crpd_name = crpd.meta().name.as_ref().unwrap();
                                        let instance_interface_name = interface_group.spec.interface_name.clone();
                                        if let Some(crpd_staus) = &crpd.status{
                                            if let Some(instance_interface) = crpd_staus.interfaces.get(&instance_interface_name){
                                                let owner_reference = match controllers::get::<core_v1::Pod>(namespace, crpd_name, ctx.client.clone()).await{
                                                    Ok(res) => {
                                                        match res{
                                                            Some((pod, _)) => {
                                                                meta_v1::OwnerReference{
                                                                    api_version: "v1".to_string(),
                                                                    kind: "Pod".to_string(),
                                                                    name: pod.meta().name.as_ref().unwrap().clone(),
                                                                    uid: pod.meta().uid.as_ref().unwrap().clone(),
                                                                    ..Default::default()
                                                                }
                                                            },
                                                            None => return Err(ReconcileError(anyhow::anyhow!("pod not found"))) 
                                                        }
                                                    },
                                                    Err(e) => return Err(e)
                                                };
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
                                                interface_spec.name = instance_interface_name.clone();
                                                interface_spec.mac = Some(instance_interface.mac.clone());
                                                let interface_name = format!("{}-{}", crpd_name, instance_interface_name);
                                                let mut interface = Interface::new(interface_name.as_str(), interface_spec);
                                                interface.metadata.namespace = Some(namespace.clone());
                                                interface.metadata.labels = Some(BTreeMap::from([
                                                    ("cnm.juniper.net/instanceSelector".to_string(), crpd.meta().name.as_ref().unwrap().clone()),
                                                    ("cnm.juniper.net/interfaceGroup".to_string(), name.clone()),
                                                    ("cnm.juniper.net/instanceType".to_string(), resources::InstanceType::Crpd.to_string()),
                                                ]));
                                                interface.metadata.owner_references = Some(vec![owner_reference]);
                                                if let Err(e) = controllers::create_or_update(interface.clone(), ctx.client.clone()).await{
                                                    return Err(e)
                                                }
                                            }
                                        }

                                    }
                                },
                                Err(e) => return Err(e)
                            }
                        }
                    }
                }
            }
        }
        match controllers::list::<Interface>(namespace, ctx.client.clone(), Some(BTreeMap::from([
            ("cnm.juniper.net/instanceSelector".to_string(), name.clone()),
            ("cnm.juniper.net/instanceType".to_string(), InstanceType::Crpd.to_string())
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
                                ..Default::default()
                            })
                        }
                        if let Err(e) = controllers::update_status(interface_group.clone(), ctx.client.clone()).await{
                            return Err(e);
                        }
                    }
                } 
            },
            Err(e) => return Err(e)
        }

        match controllers::list::<VrrpGroup>(namespace, ctx.client.clone(), Some(BTreeMap::from([
            ("cnm.juniper.net/interfaceGroup".to_string(), name.clone()),
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
                            status.vrrp_group_references = Some(ref_list);
                        } else {
                            interface_group.status = Some(InterfaceGroupStatus{
                                vrrp_group_references: Some(ref_list),
                                ..Default::default()
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
        Action::requeue(Duration::from_secs(5))
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
                Api::<CrpdGroup>::all(self.context.client.clone()),
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
                Api::<VrrpGroup>::all(self.context.client.clone()),
                Config::default(),
                |vrrp_group| {
                    info!("vrrp_group event in interface_group controller:");
                    if let Some(labels) = &vrrp_group.meta().labels{
                        if let Some(parent_group) = labels.get("cnm.juniper.net/interfaceGroup"){
                            return Some(ObjectRef::<InterfaceGroup>::new(parent_group)
                                .within(vrrp_group.meta().namespace.as_ref().unwrap()));
                        }
                    }
                    None
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
