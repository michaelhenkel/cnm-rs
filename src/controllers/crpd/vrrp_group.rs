use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::controllers::controllers;
use crate::resources::interface::{Interface, InterfaceFamily};
use crate::resources::ip_address::{IpAddress, IpAddressSpec, IpFamily};
use crate::resources::pool::{self};
use crate::resources::resources::InstanceType;
use crate::resources::vrrp::{self, InterfaceSelector, VirtualAddress, VirtualAddressAdress, VrrpUnicast, VrrpStatus, Vrrp};
use crate::resources::vrrp_group::{
    VrrpGroup,
    VrrpGroupStatus
};
use kube::core::ObjectList;
use kube::{Resource, Client};
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
use std::str::FromStr;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;
use k8s_openapi::api::core::v1 as core_v1;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta_v1;

pub struct VrrpGroupController{
    context: Arc<Context>,
    resource: Api<VrrpGroup>,
}

impl VrrpGroupController{
    pub fn new(context: Arc<Context>) -> Self{
        let resource = Api::all(context.client.clone());
        
        VrrpGroupController{context, resource}
    }
    async fn reconcile(g: Arc<VrrpGroup>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        let name = g.meta().name.as_ref().unwrap();
        let namespace = g.meta().namespace.as_ref().unwrap();
        info!("reconciling VrrpGroup {:?}", g.meta().name.as_ref().unwrap().clone());
        
        let mut vrrp_group = match controllers::get::<VrrpGroup>(namespace, name,ctx.client.clone()).await{
            Ok(res) => {
                match res{
                    Some((vrrp_group, api)) => {
                        if vrrp_group.meta().deletion_timestamp.is_none(){
                            if let Err(e) = controllers::add_finalizer(api, name).await {
                                return Err(e)
                            }
                        } else if vrrp_group.meta().deletion_timestamp.is_some() {
                            match controllers::list::<Vrrp>(namespace, ctx.client.clone(), Some(BTreeMap::from([
                                ("cnm.juniper.net/vrrpGroup".to_string(), name.clone()),
                            ]))).await{
                                Ok(res) => {
                                    if let Some((interface_list, _)) = res {
                                        for interface in &interface_list{
                                            if let Err(e) = controllers::delete::<Vrrp>(namespace.to_string(), interface.meta().name.as_ref().unwrap().clone(), ctx.client.clone()).await{
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
                        vrrp_group
                    }
                    None => { return Ok(Action::await_change()) }
                }
            },
            Err(e) => return Err(e)
        };
        
        let name = vrrp_group.meta().name.as_ref().unwrap().clone();
        let namespace = vrrp_group.meta().namespace.as_ref().unwrap().clone();

        let interface_group_parent_name = match &vrrp_group.spec.vrrp_template.interface_selector{
            InterfaceSelector::Device(_device) => { None },
            InterfaceSelector::InterfaceGroupParent(interface_group_parent) => {
                match controllers::list::<Interface>(namespace.as_str(), ctx.client.clone(), Some(BTreeMap::from([
                    ("cnm.juniper.net/interfaceGroup".to_string(), interface_group_parent.name.as_ref().unwrap().to_string())
                    ]))).await{
                    Ok(res) => {
                        if let Some((interface_list,_)) = res{
                            if let Err(e) = create_update_vrrp(interface_list, &vrrp_group, ctx.client.clone()).await{
                                return Err(e)
                            }  
                        }
                        
                    },
                    Err(e) => return Err(e)
                }
                Some(interface_group_parent.name.as_ref().unwrap().to_string())
            },
            InterfaceSelector::Selector(selector) => {
                match controllers::list::<Interface>(namespace.as_str(), ctx.client.clone(), selector.match_labels.clone()).await{
                    Ok(res) => {
                        if let Some((interface_list,_)) = res{
                            if let Err(e) = create_update_vrrp(interface_list, &vrrp_group, ctx.client.clone()).await{
                                return Err(e)
                            }
                            None
                        } else {
                            None
                        }
                    },
                    Err(e) => return Err(e)
                }
            }
        };

        if let Some(interface_group_parent_name) = interface_group_parent_name{
            if vrrp_group.meta_mut().labels.is_none(){
                vrrp_group.meta_mut().labels = Some(BTreeMap::new());
            }
            vrrp_group.meta_mut().labels.as_mut().unwrap().insert("cnm.juniper.net/interfaceGroup".to_string(), interface_group_parent_name.clone());
            if let Err(e) = controllers::create_or_update(vrrp_group.clone(), ctx.client.clone()).await{
                return Err(e)
            }
        }

        match controllers::list::<vrrp::Vrrp>(namespace.as_str(), ctx.client.clone(), Some(BTreeMap::from([
            ("cnm.juniper.net/vrrpGroup".to_string(), name.clone())
        ]))).await{
            Ok(res) => {
                if let Some((child_list,_)) = res {
                    let ref_list: Vec<core_v1::LocalObjectReference> = child_list.iter().map(|obj|{
                        core_v1::LocalObjectReference {
                            name: Some(obj.meta().name.as_ref().unwrap().clone()),
                        }
                    }).collect();
                    if ref_list.len() > 0 {
                        if let Some(status) = vrrp_group.status.as_mut(){
                            status.vrrp_references = Some(ref_list);
                        } else {
                            vrrp_group.status = Some(VrrpGroupStatus{
                                vrrp_references: Some(ref_list),
                            })
                        }
                        if let Err(e) = controllers::update_status(vrrp_group.clone(), ctx.client.clone()).await{
                            return Err(e);
                        }
                    }
                } 
            },
            Err(e) => return Err(e)
        }
        Ok(Action::await_change())
    }
    fn error_policy(_g: Arc<VrrpGroup>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}


#[async_trait]
impl Controller for VrrpGroupController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<VrrpGroup>, ctx: Arc<Context>| {
            async move { VrrpGroupController::reconcile(g, ctx).await }
        };
        let error_policy = |g: Arc<VrrpGroup>, error: &ReconcileError, ctx: Arc<Context>| {
            VrrpGroupController::error_policy(g, error, ctx)
        };
        runtime_controller::new(self.resource.clone(), Config::default())
        .watches(
            Api::<Vrrp>::all(self.context.client.clone()),
            Config::default(),
            |obj| {
                info!("vrrp event in vrrp_group controller:");
                if let Some(labels) = &obj.meta().labels{
                    if let Some(parent_group) = labels.get("cnm.juniper.net/vrrpGroup"){
                        return Some(ObjectRef::<VrrpGroup>::new(parent_group)
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
        }).await;
        Ok(())
    }
}

async fn create_update_vrrp(interface_list: ObjectList<Interface>, vrrp_group: &VrrpGroup, client: Client) -> Result<(), ReconcileError> {
    let name = vrrp_group.meta().name.as_ref().unwrap();
    let namespace = vrrp_group.meta().namespace.as_ref().unwrap();
    let mut all_interfaces_map = HashMap::new();
    for interface in &interface_list{
        if let Some(families) = &interface.spec.families{
            all_interfaces_map.insert(interface.meta().name.as_ref().unwrap().clone().to_string(), families.clone());
        }
    }
    for interface in &interface_list{
        let mut remote_v4_addresses = Vec::new();
        let mut local_v4_address = None;
        //let remote_v6_addresses = Vec::new();
        for (intf_name, remote_families) in &all_interfaces_map{
            if intf_name.clone() != interface.meta().name.as_ref().unwrap().clone(){
                if let Some(families) = &interface.spec.families{
                    for family in families{
                        match family{
                            InterfaceFamily::Inet(inet) => {
                                let local_v4_subnet = ipnet::Ipv4Net::from_str(&inet.address).unwrap().network();
                                if let Some(filter) = &vrrp_group.spec.vrrp_template.v4_subnet_filter{
                                    let filter_subnet = ipnet::Ipv4Net::from_str(filter.as_str()).unwrap().network();
                                    if filter_subnet != local_v4_subnet {
                                        continue;
                                    }
                                }
                                let local_v4_address_list: Vec<&str> = inet.address.split("/").collect();
                                local_v4_address = Some(local_v4_address_list[0].to_string());
                                
                                for remote_family in remote_families{
                                    match remote_family{
                                        InterfaceFamily::Inet(remote_inet) => {
                                            let remote_v4_subnet = ipnet::Ipv4Net::from_str(&remote_inet.address).unwrap().network();
                                            if remote_v4_subnet == local_v4_subnet{
                                                let remote_v4_addresses_list: Vec<&str> = remote_inet.address.split("/").collect();
                                                remote_v4_addresses.push(remote_v4_addresses_list[0].to_string());
                                                break;
                                            }
                                        },
                                        _ => {}
                                    }
                                }
                            },
                            InterfaceFamily::Inet6(_inet6) => {},
                        }
                    }
                }
            }
        }
        if let Some(local_v4_address) = local_v4_address {
            if remote_v4_addresses.len() > 0 {
                let virtual_address_name = format!("{}-virtual-address", name);
                let virtual_address = match get_virtual_ip_address(&vrrp_group.spec.vrrp_template.virtual_address, namespace, &virtual_address_name, client.clone()).await{
                    Ok(res) => {
                        if let Some(virtual_address) = res{
                            match virtual_address{
                                VirtualAddressAdress::Address(address) => {
                                    Some(address)
                                },
                                _ => None,
                            }
                        } else {
                            None
                        }
                    },
                    Err(e) => return Err(e)
                };

                let unicast = VrrpUnicast{
                    local_v4_address: Some(local_v4_address),
                    peer_v4_list: Some(remote_v4_addresses),
                    local_v6_address: None,
                    peer_v6_list: None,
                };

                let mut vrrp_spec = vrrp_group.spec.vrrp_template.clone();
                vrrp_spec.unicast = Some(unicast);
                vrrp_spec.instance_parent = None;

                if let Some(virtual_address) = virtual_address{
                    vrrp_spec.virtual_address = VirtualAddress{
                        v4_address: Some(VirtualAddressAdress::Address(virtual_address)),
                        v6_address: None,
                        device_name: Some(interface.spec.name.clone())
                    } 
                }
                let mut vrrp = vrrp::Vrrp::new(interface.meta().name.as_ref().unwrap(), vrrp_group.spec.vrrp_template.clone());
                vrrp.meta_mut().namespace = Some(namespace.to_string());
                vrrp.meta_mut().labels = Some(BTreeMap::from([
                    ("cnm.juniper.net/vrrpGroup".to_string(), vrrp_group.meta().name.as_ref().unwrap().to_string()),
                    ("cnm.juniper.net/instanceType".to_string(), InstanceType::Crpd.to_string()),
                    ("cnm.juniper.net/interfaceParent".to_string(), interface.meta().name.as_ref().unwrap().to_string()),
                ]));
                if let Some(interface_labels) = &interface.meta().labels{
                    if let Some(instance_selector) = interface_labels.get("cnm.juniper.net/instanceSelector"){
                        vrrp.meta_mut().labels.as_mut().unwrap().insert("cnm.juniper.net/instanceSelector".to_string(), instance_selector.to_string());
                    }
                }
                vrrp.meta_mut().owner_references = Some(vec![meta_v1::OwnerReference{
                    api_version: "cnm.juniper.net/v1".to_string(),
                    kind: "Interface".to_string(),
                    name: interface.meta().name.as_ref().unwrap().to_string(),
                    uid: interface.meta().uid.as_ref().unwrap().to_string(),
                    ..Default::default()
                }]);
                match controllers::create_or_update(vrrp.clone(), client.clone()).await{
                    Ok(res) => {
                        if let Some(mut vrrp) = res{
                            vrrp.status = Some(VrrpStatus { vrrp: Some(vrrp_spec) });
                            if let Err(e) = controllers::update_status(vrrp, client.clone()).await{
                                return Err(e);
                            }
                        }
                    },
                    Err(e) => return Err(e)
                }
            }
        }
    }
    Ok(())
}

async fn get_virtual_ip_address(virtual_address: &VirtualAddress, namespace: &str, name: &str, client: Client) -> Result<Option<VirtualAddressAdress>, ReconcileError> {
    let virtual_address = if let Some(v4_vip) = &virtual_address.v4_address{
        match v4_vip{
            VirtualAddressAdress::Address(address) => {
                Some(VirtualAddressAdress::Address(address.clone()))
            },
            VirtualAddressAdress::PoolReference(pool_ref) => {
                let ip_address = match controllers::list::<IpAddress>(namespace, client.clone(), Some(BTreeMap::from([(
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
                                    Some(VirtualAddressAdress::Address(status.address.clone()))
                                } else {
                                    return Ok(None);
                                }
                            },
                            None => return Err(ReconcileError(anyhow::anyhow!("ip address not found")))
                        }
                    },
                    None => {
                        match controllers::get::<pool::Pool>(namespace, pool_ref.name.as_ref().unwrap(), client.clone()).await{
                            Ok(res) => {
                                match res{
                                    Some((_pool, _)) => {
                                        let ip_address_spec = IpAddressSpec{
                                            pool: pool_ref.clone(),
                                            family: IpFamily::V4,
                                        };
                                        let mut ip_address = IpAddress::new(format!("{}-virtual-address", name).as_str(), ip_address_spec);
                                        ip_address.metadata.namespace = Some(namespace.to_string());
                                        if let Err(e) = controllers::create(Arc::new(ip_address), client.clone()).await{
                                            return Err(e);
                                        }
                                        return Ok(None);
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
    Ok(virtual_address)
}