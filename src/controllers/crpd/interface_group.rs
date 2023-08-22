use crate::controllers::controllers::{Controller, Context, ReconcileError};
use std::f32::consts::E;
use std::time::Duration;
use crate::controllers::controllers;
use crate::resources::interface::{Interface, InterfaceFamily, Vrrp, self, InterfaceStatus, VirtualAddress};
use crate::resources::crpd::crpd::Crpd;
use crate::resources::crpd::crpd;
use crate::resources::ip_address;
use crate::resources::pool;
use crate::resources::{vrrp, resources, interface_group};
use crate::resources::vrrp_group;

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
use std::any;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tracing::*;
use k8s_openapi::api::core::v1 as core_v1;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta_v1;
use std::str::FromStr;
use ipnet;


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
                        let mut all_instance_interface_map = BTreeMap::new();
                        for (instance_name, instance) in instance_map{
                            let instance_interface_name = interface_group.spec.interface_name.clone();
                            if let Some(instance_interface) = instance.interfaces.get(&instance_interface_name){
                                let owner_reference = match controllers::get::<core_v1::Pod>(namespace, instance_name, ctx.client.clone()).await{
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
                                let interface_name = format!("{}-{}", instance_name, instance_interface_name);
                                let mut interface = Interface::new(interface_name.as_str(), interface_spec);
                                interface.metadata.namespace = Some(namespace.clone());
                                interface.metadata.labels = Some(BTreeMap::from([
                                    ("cnm.juniper.net/instanceSelector".to_string(), crpd.meta().name.as_ref().unwrap().clone()),
                                    ("cnm.juniper.net/interfaceGroup".to_string(), name.clone()),
                                    ("cnm.juniper.net/instanceType".to_string(), resources::InstanceType::Crpd.to_string()),
                                ]));
                                interface.metadata.owner_references = Some(vec![owner_reference]);
                                match controllers::create_or_update(interface.clone(), ctx.client.clone()).await{
                                    Ok(interface) => {
                                        if let Some(interface) = interface{
                                            all_instance_interface_map.insert(instance_name.clone(), interface.clone());
                                        }
                                    },
                                    Err(e) => return Err(e)
                                }
                            }
                        }
                        let all_instance_interface_map_clone = all_instance_interface_map.clone();
                        for (instance_name, interface) in all_instance_interface_map.iter_mut(){
                            if interface.status.is_none(){
                                interface.status = Some(InterfaceStatus::default());
                            }
                            let mut peer_v4_addresses = Vec::new();
                            let mut peer_v6_addresses = Vec::new();
                            let mut local_v4_addresses = Vec::new();
                            let mut local_v6_addresses = Vec::new();
                            for (peer_instance_name, peer_interface) in &all_instance_interface_map_clone{
                                if peer_instance_name != instance_name{
                                    if let Some(families) = &peer_interface.spec.families{
                                        for family in families{
                                            match family{
                                                InterfaceFamily::Inet(inet) => {
                                                    peer_v4_addresses.push(inet.address.clone());
                                                },
                                                InterfaceFamily::Inet6(inet6) => {
                                                    peer_v6_addresses.push(inet6.address.clone());
                                                }
                                            }
                                        }
                                    }
                                }else if let Some(families) = &interface.spec.families{
                                    for family in families{
                                        match family{
                                            InterfaceFamily::Inet(inet) => {
                                                local_v4_addresses.push(inet.address.clone());
                                            },
                                            InterfaceFamily::Inet6(inet6) => {
                                                local_v6_addresses.push(inet6.address.clone());
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(vrrp) = interface.spec.vrrp.as_mut(){
                                let mut found_local_v4_address = None;
                                let mut found_peer_v4_addresses = Vec::new();
                                let mut found_local_v6_address = None;
                                let mut found_peer_v6_addresses = Vec::new();
                                for local_v4_address in &local_v4_addresses{
                                    let local_v4_subnet = ipnet::Ipv4Net::from_str(local_v4_address).unwrap().network();
                                    if let Some(filter) = &vrrp.v4_subnet_filter{
                                        let filter_subnet = ipnet::Ipv4Net::from_str(filter.as_str()).unwrap().network();
                                        if filter_subnet != local_v4_subnet {
                                            continue;
                                        }
                                    }
                                    for peer_v4_address in &peer_v4_addresses{
                                        let peer_v4_subnet = ipnet::Ipv4Net::from_str(peer_v4_address).unwrap().network();
                                        if peer_v4_subnet == local_v4_subnet {
                                            found_local_v4_address = Some(local_v4_address.clone());
                                            found_peer_v4_addresses.push(peer_v4_address.clone());
                                            break;
                                        }
                                    }
                                }
                                for local_v6_address in &local_v6_addresses{
                                    let local_v6_subnet = ipnet::Ipv6Net::from_str(local_v6_address).unwrap().network();
                                    for peer_v6_address in &peer_v6_addresses{
                                        let peer_v6_subnet = ipnet::Ipv6Net::from_str(peer_v6_address).unwrap().network();
                                        if let Some(filter) = &vrrp.v6_subnet_filter{
                                            let filter_subnet = ipnet::Ipv6Net::from_str(filter.as_str()).unwrap().network();
                                            if filter_subnet != local_v6_subnet {
                                                continue;
                                            }
                                        }
                                        if peer_v6_subnet == local_v6_subnet {
                                            found_local_v6_address = Some(local_v6_address.clone());
                                            found_peer_v6_addresses.push(peer_v6_address.clone());
                                            break;
                                        }
                                    }
                                }
                                let mut unicast: Option<interface::VrrpUnicast> = None;
                                if let Some(local_v4_address) = found_local_v4_address{
                                    if unicast.is_none(){
                                        unicast = Some(interface::VrrpUnicast{
                                            local_v4_address: Some(local_v4_address),
                                            peer_v4_list: Some(peer_v4_addresses),
                                            local_v6_address: None,
                                            peer_v6_list: None,
                                        })
                                    } else {
                                        unicast.as_mut().unwrap().local_v4_address = Some(local_v4_address);
                                        unicast.as_mut().unwrap().peer_v4_list = Some(peer_v4_addresses);
                                    }
                                }
                                if let Some(local_v6_address) = found_local_v6_address{
                                    if unicast.is_none(){
                                        unicast = Some(interface::VrrpUnicast{
                                            local_v4_address: None,
                                            peer_v4_list: None,
                                            local_v6_address: Some(local_v6_address),
                                            peer_v6_list: Some(peer_v6_addresses),
                                        })
                                    } else {
                                        unicast.as_mut().unwrap().local_v6_address = Some(local_v6_address);
                                        unicast.as_mut().unwrap().peer_v6_list = Some(peer_v6_addresses);
                                    }
                                }

                                let virtual_address = if let Some(v4_vip) = &vrrp.virtual_address.v4_address{
                                    match v4_vip{
                                        interface::VirtualAddressAdress::Address(address) => {
                                            Some(interface::VirtualAddressAdress::Address(address.clone()))
                                        },
                                        interface::VirtualAddressAdress::PoolReference(pool_ref) => {
                                            let ip_address = match controllers::list::<ip_address::IpAddress>(namespace, ctx.client.clone(), Some(BTreeMap::from([(
                                                "cnm.juniper.net/pool".to_string(),pool_ref.name.as_ref().unwrap().clone() 
                                            )]))).await{
                                                Ok(res) => {
                                                    match res{
                                                        Some((ip_address_list,_)) => {
                                                            let mut found_ip_address = None;
                                                            for ip_address in &ip_address_list{
                                                                let ip_address_name = ip_address.meta().name.as_ref().unwrap();
                                                                let vrrp_virtual_address_name = format!("{}-virtual-address", name);
                                                                if ip_address_name.to_string() == vrrp_virtual_address_name{
                                                                    found_ip_address = Some(ip_address.clone());
                                                                    break;
                                                                }
                                                            }
                                                            found_ip_address
                                                        },
                                                        None => None
                                                    }
                                                },
                                                Err(e) => return Err(e)
                                            };
                                            match ip_address{
                                                Some(ip_address) => {
                                                    match ip_address.status{
                                                        Some(status) => {
                                                            if !status.address.is_empty(){
                                                                Some(interface::VirtualAddressAdress::Address(status.address.clone()))
                                                            } else {
                                                                return Ok(Action::requeue(Duration::from_secs(1)));
                                                            }
                                                        },
                                                        None => return Err(ReconcileError(anyhow::anyhow!("ip address not found")))
                                                    }
                                                },
                                                None => {
                                                    match controllers::get::<pool::Pool>(namespace, pool_ref.name.as_ref().unwrap(), ctx.client.clone()).await{
                                                        Ok(res) => {
                                                            match res{
                                                                Some((_pool, _)) => {
                                                                    let ip_address_spec = ip_address::IpAddressSpec{
                                                                        pool: pool_ref.clone(),
                                                                        family: ip_address::IpFamily::V4,
                                                                    };
                                                                    let mut ip_address = ip_address::IpAddress::new(format!("{}-virtual-address", name).as_str(), ip_address_spec);
                                                                    ip_address.metadata.namespace = Some(namespace.clone());
                                                                    if let Err(e) = controllers::create(Arc::new(ip_address), ctx.client.clone()).await{
                                                                        return Err(e);
                                                                    }
                                                                    return Ok(Action::requeue(Duration::from_secs(1)));
                                                                },
                                                                None => return Err(ReconcileError(anyhow::anyhow!("pool not found")))
                                                            }
                                                        },
                                                        Err(e) => return Err(e)
                                                    }
                                                },
                                            }
                                        }
                                    }
                                } else {
                                    None
                                };
                                if virtual_address.is_some() || unicast.is_some(){
                                    let mut vrrp_status = Vrrp::default();
                                    if let Some(virtual_address) = virtual_address{
                                        vrrp_status.virtual_address = VirtualAddress{
                                            v4_address: Some(virtual_address),
                                            ..Default::default()
                                        };
                                    }
                                    vrrp_status.unicast = unicast;
                                    interface.status.as_mut().unwrap().vrrp = Some(vrrp_status);
                                    if let Err(e) = controllers::update_status(interface.clone(), ctx.client.clone()).await{
                                        return Err(e)
                                    }

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
