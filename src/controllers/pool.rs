use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::controllers::controllers;
use crate::resources::pool::{
    Pool,
    PoolType,
    PoolStatus
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
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;
use std::str::FromStr;

pub struct PoolController{
    context: Arc<Context>,
    resource: Api<Pool>,
}

impl PoolController{
    pub fn new(context: Arc<Context>) -> Self{
        let resource = Api::all(context.client.clone());
        
        PoolController{context, resource}
    }
    async fn reconcile(g: Arc<Pool>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        info!("reconciling Pool {:?}", g.meta().name.as_ref().unwrap().clone());
        
        match controllers::get::<Pool>(
            g.meta().namespace.as_ref().unwrap(),
            g.meta().name.as_ref().unwrap(),
            ctx.client.clone())
            .await{
            Ok(res) => {
                match res{
                    Some((mut pool, _api)) => {
                        if pool.status.is_none(){
                            match &pool.spec.pool_type{
                                PoolType::V4{
                                    prefix,
                                    length,
                                } => {
                                    let prefix = prefix.clone();
                                    let sn = std::net::Ipv4Addr::from_str(prefix.as_str()).unwrap();
                                    let sn = sn.octets();
                                    let be = as_u32_be(&sn);
                                    let rev_length = 32 - *length as u32;       
                                    let max_size = be | rev_length;
                                
                                    let status = PoolStatus{
                                        max_size: max_size as u128,
                                        length: *length,
                                        in_use: 0,
                                        next_available: be as u128,
                                        released_numbers: BTreeMap::new(),
                                    };
                                    pool.status = Some(status);
                                    if let Err(e) = controllers::update_status(pool, ctx.client.clone()).await{
                                        return Err(e);
                                    }

                                },
                                PoolType::V6{
                                    prefix: _,
                                    length: _,
                                } => {},
                                PoolType::RouteTarget {
                                    start: _,
                                    size: _,
                                } => {},
                            }

                        }
                    }
                    None => {}
                }
            },
            Err(e) => return Err(e)
            
        }
    
        Ok(Action::await_change())
    }
    fn error_policy(_g: Arc<Pool>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

#[async_trait]
impl Controller for PoolController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<Pool>, ctx: Arc<Context>| {
            async move { PoolController::reconcile(g, ctx).await }
        };
        let error_policy = |g: Arc<Pool>, error: &ReconcileError, ctx: Arc<Context>| {
            PoolController::error_policy(g, error, ctx)
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

pub fn as_u32_be(array: &[u8; 4]) -> u32 {
    ((array[0] as u32) << 24) +
    ((array[1] as u32) << 16) +
    ((array[2] as u32) <<  8) +
    ((array[3] as u32) <<  0)
}

pub fn as_u128_be(array: &[u8; 16]) -> u128 {
    ((array[0] as u128) << 120) +
    ((array[1] as u128) << 112) +
    ((array[2] as u128) << 104) +
    ((array[3] as u128) << 96) +
    ((array[4] as u128) << 88) +
    ((array[5] as u128) << 80) +
    ((array[6] as u128) << 72) +
    ((array[7] as u128) << 64) +
    ((array[8] as u128) << 56) +
    ((array[9] as u128) << 48) +
    ((array[10] as u128) << 40) +
    ((array[11] as u128) << 32) +
    ((array[12] as u128) << 24) +
    ((array[13] as u128) << 16) +
    ((array[14] as u128) <<  8) +
    ((array[15] as u128) <<  0)
}
