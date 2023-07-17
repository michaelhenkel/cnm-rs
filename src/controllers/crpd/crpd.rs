use crate::controllers::controllers::{Controller, Context, ReconcileError, self};
use crate::resources::crpd::crpd::Crpd;
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
use k8s_openapi::api::apps::v1 as apps_v1;

pub struct CrpdController{
    context: Arc<Context>,
    resource: Api<Crpd>,
}

impl CrpdController{
    pub fn new(client: Client) -> Self{
        let resource = Api::all(client.clone());
        let context = Arc::new(Context{
            client: client.clone(),
        });
        CrpdController{context, resource}
    }
    async fn reconcile(g: Arc<Crpd>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        match controllers::get_resource(g.clone(), ctx.client.clone()).await{
            Ok(res) => {
                match res{
                    Some((crpd, crpd_api)) => {
                        let deployment = apps_v1::Deployment{
                            metadata: crpd.metadata.clone(),
                            ..Default::default()
                        };
                        match controllers::get_resource(Arc::new(deployment), ctx.client.clone()).await{
                            Ok(res) => {
                                match res{
                                    Some((deployment, deployment_api)) => {
                                        info!("deployment exists");
                                        //Ok(Action::Continue)
                                    },
                                    None => {
                                        info!("deployment does not exist");
                                        //Ok(Action::await_change())
                                    },
                                }
                            },
                            Err(e) => {
                                return Err(e);
                            },
                        }
                        Ok(Action::await_change())
                    },
                    None => {
                        info!("crpd does not exist");
                        Ok(Action::await_change())
                    },
                }
            },
            Err(e) => {
                return Err(e);
            },
        }
    }
    fn error_policy(g: Arc<Crpd>, error: &ReconcileError, ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

#[async_trait]
impl Controller for CrpdController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<Crpd>, ctx: Arc<Context>| {
            async move { CrpdController::reconcile(g, ctx).await }
        };
        let error_policy = |g: Arc<Crpd>, error: &ReconcileError, ctx: Arc<Context>| {
            CrpdController::error_policy(g, error, ctx)
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
