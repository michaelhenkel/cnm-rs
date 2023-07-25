use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::controllers::controllers;
use crate::cert;
use crate::controllers::crpd::junos;
use crate::resources::bgp_router::BgpRouter;
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
use std::f32::consts::E;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;
use k8s_openapi::api::core::v1 as core_v1;

pub struct JunosConfigurationController{
    context: Arc<Context>,
    resource: Api<BgpRouter>,
}

impl JunosConfigurationController{
    pub fn new(context: Arc<Context>) -> Self{
        let resource = Api::all(context.client.clone());
        let context = context.clone();
        JunosConfigurationController{context, resource}
    }
    async fn reconcile(g: Arc<BgpRouter>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        match controllers::get::<BgpRouter>(
            g.meta().namespace.as_ref().unwrap().clone(),
            g.meta().name.as_ref().unwrap().clone(),
            ctx.client.clone())
            .await{
            Ok(res) => {
                match res{
                    Some((bgp_router, _)) => {
                        info!("junos config controller reconciles bgprouter config");
                        if let Some(address) = &bgp_router.spec.address{
                            match junos::client::Client::new(
                                address.clone(),
                                bgp_router.meta().name.as_ref().unwrap().clone(),
                                ctx.key.as_ref().unwrap().clone(),
                                ctx.ca.as_ref().unwrap().clone(),
                                ctx.cert.as_ref().unwrap().clone()).await{
                                Ok(mut client) => {
                                    match client.get().await{
                                        Ok(config) => {
                                            info!("JUNOS config: {:#?}", config);
                                        },
                                        Err(e) => { return Err(ReconcileError(e.into()))}
                                    }
                                },
                                Err(e) => {
                                    return Err(ReconcileError(e.into()))
                                },
                            }
                        }
                        if let Some(status) = bgp_router.status{
                            
                        }
                    },
                    None => {}
                }
            },
            Err(e) => {
                return Err(e);
            }
        }
        Ok(Action::await_change())
    }
    fn error_policy(g: Arc<BgpRouter>, error: &ReconcileError, ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

#[async_trait]
impl Controller for JunosConfigurationController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<BgpRouter>, ctx: Arc<Context>| {
            async move { JunosConfigurationController::reconcile(g, ctx).await }
        };
        let error_policy = |g: Arc<BgpRouter>, error: &ReconcileError, ctx: Arc<Context>| {
            JunosConfigurationController::error_policy(g, error, ctx)
        };
        let mut config = Config::default();
        config.label_selector = Some("
            cnm.juniper.net/bgpRouterManaged=true,
            cnm.juniper.net/bgpRouterType=Crpd
        ".to_string());
        runtime_controller::new(self.resource.clone(), config)
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
