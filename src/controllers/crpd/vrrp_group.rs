use crate::controllers::controllers::{Controller, Context, ReconcileError};

use crate::controllers::controllers;
use crate::resources::ip_address::{IpAddress, IpAddressSpec, IpFamily};
use crate::resources::pool::Pool;
use crate::resources::vrrp;
use crate::resources::interface;
use crate::resources::interface_group;
use crate::resources::vrrp::InterfaceSelector;
use kube::api::ObjectList;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;

use crate::resources::vrrp_group::{
    VrrpGroup,
    VrrpGroupStatus
};
use garde::rules::ip;
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
use std::any;
use std::collections::BTreeMap;
use std::f32::consts::E;
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
        
        let (mut vrrp_group, _vrrp_group_api) = match controllers::get::<VrrpGroup>(namespace, name,ctx.client.clone()).await{
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

        let mut vrrp_spec = vrrp_group.spec.vrrp_template.clone();
        let virtual_ip_address = match &vrrp_spec.virtual_address.address{
            vrrp::VirtualAddressAdress::PoolReference(pool_ref) => {
                let ip_address = match controllers::list::<IpAddress>(namespace, ctx.client.clone(), Some(BTreeMap::from([(
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
                                    status.address.clone()
                                } else {
                                    return Ok(Action::requeue(Duration::from_secs(1)));
                                }
                            },
                            None => {
                                return Err(ReconcileError(anyhow::anyhow!("ip address not found")))
                            }
                        }
                    },
                    None => {
                        match controllers::get::<Pool>(namespace, pool_ref.name.as_ref().unwrap(), ctx.client.clone()).await{
                            Ok(res) => {
                                match res{
                                    Some((_pool, _)) => {
                                        let ip_address_spec = IpAddressSpec{
                                            pool: pool_ref.clone(),
                                            family: IpFamily::V4,
                                        };
                                        let mut ip_address = IpAddress::new(format!("{}-virtual-address", name).as_str(), ip_address_spec);
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
            },
            vrrp::VirtualAddressAdress::Address(ip_address) => ip_address.clone(),
        };
        match &vrrp_spec.interface_selector{
            InterfaceSelector::Selector(selector) => {
                match controllers::list::<interface::Interface>(namespace, ctx.client.clone(), selector.match_labels.clone()).await{
                    Ok(interface_list) => {
                        if let Err(e) = create_vrrp(interface_list, vrrp_spec, ctx.client.clone(), name.clone(), virtual_ip_address).await{
                            return Err(e)
                        }
                    },
                    Err(e) => return Err(e)
                }
            },
            InterfaceSelector::InterfaceGroupParent(interface_group_ref) => {
                match controllers::list::<interface::Interface>(namespace, ctx.client.clone(), Some(BTreeMap::from([(
                    "cnm.juniper.net/interfaceGroup".to_string(), interface_group_ref.name.as_ref().unwrap().clone()
                )]))).await{
                    Ok(interface_list) => {
                        if let Err(e) = create_vrrp(interface_list, vrrp_spec, ctx.client.clone(), name.clone(), virtual_ip_address).await{
                            return Err(e)
                        }

                    },
                    Err(e) => return Err(e)
                }

            },
            InterfaceSelector::Device(_device) => {  },
        };

        

        if let Err(e) = controllers::update_status(vrrp_group, ctx.client.clone()).await{
            return Err(e);
        }
    
        Ok(Action::await_change())
    }
    fn error_policy(_g: Arc<VrrpGroup>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

async fn create_vrrp(
    interface_list: Option<(ObjectList<interface::Interface>, Api<interface::Interface>)>,
    mut vrrp_spec: vrrp::VrrpSpec,
    client: Client,
    name: String,
    virtual_ip_address: String
) -> Result<(), ReconcileError>{
    if let Some((interface_list, _)) = interface_list{
        for intf in &interface_list{
            let intf_name = intf.meta().name.as_ref().unwrap();
            let intf_namespace = intf.meta().namespace.as_ref().unwrap();
            let mut local_address = None;
            if vrrp_spec.unicast.is_none(){
                match &intf.spec.families{
                    Some(families) => {
                        for family in families{
                            match family{
                                interface::InterfaceFamily::Inet(inet) => {
                                    local_address = Some(inet.address.clone());
                                    break;
                                },
                                _ => {}
                            }
                        }
                    },
                    None => {
                        return Err(ReconcileError(anyhow::anyhow!("interface has no address")))
                    }
                };
            } else {
                local_address = Some(vrrp_spec.unicast.as_ref().unwrap().local_address.clone())
            }
            if local_address.is_none(){
                return Err(ReconcileError(anyhow::anyhow!("interface has no address")))
            }
            if vrrp_spec.virtual_address.device_name.is_none(){
                vrrp_spec.virtual_address.device_name = Some(intf_name.clone());
            }
            
            let mut peers_configured = false;
            match &vrrp_spec.unicast{
                Some(unicast) => {
                    if unicast.peer_address.is_some(){
                        peers_configured = true;
                    }
                },
                None => {}
            }

            if !peers_configured{
                let mut peer_address_list = Vec::new();
                for peer_intf in &interface_list{
                    let peer_intf_name = peer_intf.meta().name.as_ref().unwrap();
                    if peer_intf_name != intf_name{
                        if let Some(families) = &peer_intf.spec.families{
                            for family in families{
                                match family{
                                    interface::InterfaceFamily::Inet(inet) => {
                                        peer_address_list.push(inet.address.clone());
                                    },
                                    _ => {}
                                }
                            }
                        }
                    }
                }

                vrrp_spec.unicast = Some(vrrp::VrrpUnicast{
                    local_address: local_address.as_ref().unwrap().clone(),
                    peer_address: if peer_address_list.len() > 0{
                        Some(peer_address_list)
                    } else {
                        None
                    }
                });                
                
            }

            let mut vrrp = vrrp::Vrrp::new(format!("{}-{}", intf_name, name).as_str(), vrrp_spec.clone());
            vrrp.metadata.namespace = Some(intf_namespace.clone());
            if vrrp.metadata.labels.is_none(){
                vrrp.metadata.labels = Some(BTreeMap::new());
            }
            vrrp.metadata.owner_references = Some(vec![OwnerReference{
                api_version: "cnm.juniper.net/v1".to_string(),
                kind: "Interface".to_string(),
                name: intf_name.clone(),
                uid: intf.meta().uid.as_ref().unwrap().clone(),
                controller: Some(false),
                block_owner_deletion: Some(false)
            }]);
            vrrp.metadata.labels.as_mut().unwrap().insert("cnm.juniper.net/interfaceGroup".to_string(), name.clone());
            vrrp.metadata.labels.as_mut().unwrap().insert("cnm.juniper.net/interface".to_string(), intf_name.clone());
            let vrrp_status = vrrp::VrrpStatus{
                virtual_address: virtual_ip_address.clone()
            };
            
            match controllers::create_or_update::<vrrp::Vrrp>(vrrp.clone(), client.clone()).await{
                Ok(vrrp) => {
                    if let Some(mut vrrp) = vrrp{
                        vrrp.status = Some(vrrp_status);
                        if let Err(e) = controllers::update_status::<vrrp::Vrrp>(vrrp.clone(), client.clone()).await{
                            return Err(e);
                        }
                    }
                },
                Err(e) => return Err(e)
            }
        }
    }
    Ok(())
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
            Api::<interface_group::InterfaceGroup>::all(self.context.client.clone()),
            Config::default(),
            |interface_group| {
                info!("interface_group event in vrrp_group controller:");
                let mut object_list = Vec::new();
                match &interface_group.status{
                    Some(status) => {
                        for vrrp_group_ref in &status.vrrp_group_references{
                            let object = ObjectRef::<VrrpGroup>::new(
                                vrrp_group_ref.name.as_ref().unwrap().clone().as_str())
                                .within(interface_group.meta().namespace.as_ref().unwrap());
                            object_list.push(object);
                        }
                    },
                    None => {}
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
        }).await;
        Ok(())
    }
}
