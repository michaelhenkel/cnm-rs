use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::controllers::controllers;
use crate::resources::vrrp::Vrrp;
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

use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;

pub struct VrrpController{
    context: Arc<Context>,
    resource: Api<Vrrp>,
}

impl VrrpController{
    pub fn new(context: Arc<Context>) -> Self{
        let resource = Api::all(context.client.clone());
        
        VrrpController{context, resource}
    }
    async fn reconcile(g: Arc<Vrrp>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        let name = g.meta().name.as_ref().unwrap();
        let namespace = g.meta().namespace.as_ref().unwrap();
        info!("reconciling Vrrp {:?}", g.meta().name.as_ref().unwrap().clone());
        
        match controllers::get::<Vrrp>(namespace, name,ctx.client.clone()).await{
            Ok(res) => {
                match res{
                    Some((_interface, _api)) => {
                        
                    }
                    None => {}
                }
            },
            Err(e) => return Err(e)
            
        }
    
        Ok(Action::await_change())
    }
    fn error_policy(_g: Arc<Vrrp>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

#[async_trait]
impl Controller for VrrpController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<Vrrp>, ctx: Arc<Context>| {
            async move { VrrpController::reconcile(g, ctx).await }
        };
        let error_policy = |g: Arc<Vrrp>, error: &ReconcileError, ctx: Arc<Context>| {
            VrrpController::error_policy(g, error, ctx)
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
