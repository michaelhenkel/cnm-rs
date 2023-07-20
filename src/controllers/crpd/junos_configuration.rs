use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::controllers::controllers;
use crate::controllers::crpd::junos::junos;
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
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;

pub struct JunosConfigurationController{
    context: Arc<Context>,
    resource: Api<BgpRouter>,
}

impl JunosConfigurationController{
    pub fn new(client: Client) -> Self{
        let resource = Api::all(client.clone());
        let context = Arc::new(Context{
            client: client.clone(),
        });
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
                    Some((bgp_router, _)) => {},
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
        config.field_selector = Some("cnm.juniper.net/bgpRouterManaged=bgp-true,
        cnm.juniper.net/bgpRouterType=Crpd".to_string());
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
