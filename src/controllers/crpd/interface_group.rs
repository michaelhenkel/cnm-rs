use crate::controllers::controllers::{Controller, Context, ReconcileError};

use crate::controllers::controllers;
use crate::resources::interface::Interface;
use crate::resources::crpd::crpd::Crpd;
use crate::resources::vrrp;
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
use std::collections::BTreeMap;
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
        info!("reconciling InterfaceGroup {:?}", g.meta().name.as_ref().unwrap().clone());
        
        let (mut interface_group, _interface_group_api) = match controllers::get::<InterfaceGroup>(namespace, name,ctx.client.clone()).await{
            Ok(res) => {
                match res{
                    Some((interface, interface_api)) => {
                            (interface, interface_api)
                    }
                    None => { return Ok(Action::await_change()) }
                }
            },
            Err(e) => return Err(e)
        };

        let crpd = match controllers::get::<Crpd>(
            namespace,
            interface_group.spec.interface_template.instance_parent.reference.name.as_ref().unwrap(), 
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
            let mut interface_references = Vec::new();
            if let Some(status) = &crpd.status{
                if let Some(instances) = &status.instances{
                    for (instance_name, instance) in instances{
                        let intf_map = match &interface_group.spec.interface_selector{
                            InterfaceSelector::All(_) => {
                                instance.interfaces.clone()
                            },
                            InterfaceSelector::List(interface_list) => {
                                let mut intf_list = BTreeMap::new();
                                
                                for intf in interface_list{
                                    if let Some(inst_intf) = instance.interfaces.get(intf){
                                        intf_list.insert(intf.clone(), inst_intf.clone());
                                    }
                                }
                                
                                intf_list
                            }
                        };

                        let pod = match controllers::get::<core_v1::Pod>(
                            namespace,
                            instance_name,
                            ctx.client.clone()).await{
                                Ok(res) => {
                                    match res{
                                        Some((pod, _api)) => {
                                            pod
                                        },
                                        None => return Ok(Action::requeue(Duration::from_secs(5)))
                                    }
                                },
                                Err(e) => return Err(e)
                            };

                        for (inst_intf_name, inst_intf) in intf_map{
                            let interface_name = format!("{}-{}", instance_name, inst_intf_name.clone());
                            let mut interface_spec = interface_group.spec.interface_template.clone();
                            interface_spec.name = Some(inst_intf_name);
                            interface_spec.instance_parent.reference.name = Some(instance_name.clone());
                            interface_spec.mtu = Some(8900);
                            interface_spec.mac = Some(inst_intf.mac.clone());
                            let mut family_list = Vec::new();
                            
                            
                            if let Some(v4) = inst_intf.v4_address{
                                let interface_family = interface::InterfaceFamily::new(v4.as_str());
                                family_list.push(interface_family);
                            }

                            if let Some(v6) = inst_intf.v6_address{
                                let interface_family = interface::InterfaceFamily::new(v6.as_str());
                                family_list.push(interface_family);
                            }

                            interface_spec.families = Some(family_list);

                            let mut interface = interface::Interface::new(&interface_name, interface_spec);
                            interface.metadata.namespace = Some(namespace.clone());
                            interface.metadata.labels = Some(
                                BTreeMap::from([
                                    ("cnm.juniper.net/interfaceGroup".to_string(), name.clone()),
                                    ("cnm.juniper.net/instanceSelector".to_string(), crpd.metadata.name.as_ref().unwrap().clone())
                                ])
                            );
                            interface.metadata.owner_references = Some(
                                vec![meta_v1::OwnerReference{
                                    api_version: "v1".to_string(),
                                    block_owner_deletion: Some(false),
                                    controller: Some(false),
                                    kind: "Pod".to_string(),
                                    name: pod.meta().name.as_ref().unwrap().clone(),
                                    uid: pod.meta().uid.as_ref().unwrap().clone(),
                                    ..Default::default()
                                }]
                            );

                            if let Err(e) = controllers::create_or_update(interface, ctx.client.clone()).await{
                                return Err(e);
                            }

                            interface_references.push(
                                core_v1::LocalObjectReference{
                                    name: Some(interface_name)
                                }
                            )
                            
                        }
                        
                    }
                }
            }

            let mut vrrp_group_references = Vec::new();
            match controllers::list::<vrrp_group::VrrpGroup>(
                namespace,
                ctx.client.clone(),
                Some(BTreeMap::from([(
                    "cnm.juniper.net/interfaceGroup".to_string(), name.clone()
                )]))).await{
                Ok(res) => {
                    if let Some((vrrp_group_list, _)) = res {
                        for vrrp_group in vrrp_group_list{
                            vrrp_group_references.push(
                                core_v1::LocalObjectReference{
                                    name: Some(vrrp_group.meta().name.as_ref().unwrap().clone())
                                }
                            )
                        }
                    }
                },
                Err(e) => return Err(e)
            };


            if let Some(status) = interface_group.status.as_mut(){
                status.interface_references = interface_references;
                status.vrrp_group_references = vrrp_group_references;
            } else {
                let status = InterfaceGroupStatus{
                    interface_references,
                    vrrp_group_references
                };
                interface_group.status = Some(status);
            }

            if let Err(e) = controllers::update_status(interface_group, ctx.client.clone()).await{
                return Err(e);
            }
    
        Ok(Action::await_change())
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
                info!("crpd event in interface_group controller:");
                let mut object_list = Vec::new();
                match crpd.status{
                    Some(status) => {
                        match status.interface_group_references{
                            Some(interface_group_refs) => {
                                for interface_group_ref in &interface_group_refs{
                                    let object = ObjectRef::<InterfaceGroup>::new(
                                        interface_group_ref.name.as_ref().unwrap().clone().as_str())
                                        .within(interface_group_ref.namespace.as_ref().unwrap());
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
            Api::<Interface>::all(self.context.client.clone()),
            Config::default(),
            |interface| {
                info!("interface event in interface_group controller:");
                match &interface.meta().labels{
                    Some(labels) => {
                        match labels.get("cnm.juniper.net/interfaceGroup"){
                            Some(interface_group_name) => {
                                Some(ObjectRef::<InterfaceGroup>::new(
                                    interface_group_name)
                                    .within(interface.meta().namespace.as_ref().unwrap()))
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
            Api::<vrrp_group::VrrpGroup>::all(self.context.client.clone()),
            Config::default(),
            |vrrp_group| {
                info!("vrrp_group event in interface_group controller:");
                match &vrrp_group.meta().labels{
                    Some(labels) => {
                        match labels.get("cnm.juniper.net/interfaceGroup"){
                            Some(interface_group_name) => {
                                Some(ObjectRef::<InterfaceGroup>::new(
                                    interface_group_name)
                                    .within(vrrp_group.meta().namespace.as_ref().unwrap()))
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
