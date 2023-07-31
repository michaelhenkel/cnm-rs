use crate::controllers::controllers::{Controller,
    Context,
    ReconcileError,
    action,
    add_finalizer,
    del_finalizer,
    ReconcileAction,
};
use crate::controllers::crpd::junos::bgp;
use crate::controllers::{controllers, bgp_router, pool as pool_controller};
use crate::resources::pool;
use crate::resources::ip_address::{
    IpAddress,
    IpAddressStatus,
    IpFamily,
};
use kube::Resource;
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
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;
use k8s_openapi::api::core::v1 as core_v1;
use std::str::FromStr;


pub struct IpAddressController{
    context: Arc<Context>,
    resource: Api<IpAddress>,
}

impl IpAddressController{
    pub fn new(context: Arc<Context>) -> Self{
        let resource = Api::all(context.client.clone());
        
        IpAddressController{context, resource}
    }
    async fn reconcile(g: Arc<IpAddress>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        info!("reconciling IpAddress {:?}", g.meta().name.as_ref().unwrap().clone());
        let name = g.meta().name.as_ref().unwrap().clone();
        let namespace = g.meta().namespace.as_ref().unwrap().clone();
        let ip = match controllers::get::<IpAddress>(
            g.meta().namespace.as_ref().unwrap().clone(),
            g.meta().name.as_ref().unwrap().clone(),
            ctx.client.clone())
            .await{
            Ok(res) => {
                match res{
                    Some((mut ip_address, ip_address_api)) => {
                        if ip_address.status.is_none() || ip_address.status.as_ref().unwrap().address.is_empty(){
                            match controllers::get::<pool::Pool>(
                                ip_address.meta().namespace.as_ref().unwrap().clone(),
                                ip_address.spec.pool.name.as_ref().unwrap().clone(),
                                ctx.client.clone())
                                .await{
                                Ok(res) => {
                                    match res{
                                        Some((mut pool, _api)) => {
                                            let (ip, length) = match ip_address.spec.family{
                                                IpFamily::V4 => {
                                                    match pool.spec.pool_type{
                                                        pool::PoolType::V4{
                                                            prefix: _,
                                                            length} => {
                                                                let ip = match pool.status.as_mut(){
                                                                    Some(status) => {
                                                                        match status.assign_number(){
                                                                            Some(ip) => ip,
                                                                            None => return Err(ReconcileError(anyhow::anyhow!("no ip address available")))
                                                                        }
                                                                    },
                                                                    None => return Err(ReconcileError(anyhow::anyhow!("no pool status available")))
                                                                };
                                                                let ip = ip as u32;
                                                                (std::net::Ipv4Addr::from(ip.to_be_bytes()).to_string(), length)
                                                        },
                                                        _ => return Err(ReconcileError(anyhow::anyhow!("wrong pool type")))
                                                    }
                                                }
                                                IpFamily::V6 => {
                                                    match pool.spec.pool_type{
                                                        pool::PoolType::V6{
                                                            prefix: _,
                                                            length} => {
                                                                ("".to_string(), length)
                                                        },
                                                        _ => return Err(ReconcileError(anyhow::anyhow!("wrong pool type")))
                                                    }
                                                }
                                            };
                                            if let Err(e) = controllers::update_status(pool, ctx.client.clone()).await{
                                                return Err(e)
                                            }
                                            ip_address.status = Some(IpAddressStatus{
                                                address: format!("{}/{}", ip, length),
                                            });
                                            if let Err(e) = controllers::update_status(ip_address.clone(), ctx.client.clone()).await{
                                                return Err(e)
                                            }
                                            
                                        }
                                        None => {}
                                    }
                                },
                                Err(e) => return Err(e)
                            };
                        }
                        Some((ip_address, ip_address_api))
                    }
                    None => None
                }
            },
            Err(e) => return Err(e)
        };

        let (ip_address, ip_address_api) = if let Some(ip) = ip{
            ip
        } else {
            return Ok(Action::await_change());
        };
        //Ok(Action::await_change())
        return match action(&ip_address) {
            ReconcileAction::Create => {
                controllers::add_finalizer(ip_address_api.clone(), &name).await?;
                Ok(Action::await_change())
            }
            ReconcileAction::Delete => {
                match controllers::get::<pool::Pool>(
                    ip_address.meta().namespace.as_ref().unwrap().clone(),
                    ip_address.spec.pool.name.as_ref().unwrap().clone(),
                    ctx.client.clone())
                    .await{
                    Ok(res) => {
                        match res{
                            Some((mut pool, _api)) => {
                                let ip = ip_address.status.as_ref().unwrap().address.clone();
                                let ip = std::net::Ipv4Addr::from_str(&ip).unwrap();
                                let ip = pool_controller::as_u32_be(&ip.octets());
                                pool.status.as_mut().unwrap().return_number(ip as u128);
                                if let Err(e) = controllers::update_status(pool, ctx.client.clone()).await{
                                    return Err(e)
                                }
                            },
                            None => {}
                        }
                    },
                    Err(e) => return Err(e)
                };
                controllers::del_finalizer(ip_address_api.clone(), &name).await?;
                Ok(Action::await_change())
            }
            ReconcileAction::NoOp => Ok(Action::await_change()),
        };
    }
    fn error_policy(g: Arc<IpAddress>, error: &ReconcileError, ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

#[async_trait]
impl Controller for IpAddressController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<IpAddress>, ctx: Arc<Context>| {
            async move { IpAddressController::reconcile(g, ctx).await }
        };
        let error_policy = |g: Arc<IpAddress>, error: &ReconcileError, ctx: Arc<Context>| {
            IpAddressController::error_policy(g, error, ctx)
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
