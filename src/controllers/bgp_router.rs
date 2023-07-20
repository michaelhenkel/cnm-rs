use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::resources::bgp_router::BgpRouter;
use crate::resources::bgp_router_group::BgpRouterGroup;
use kube::runtime::reflector::ObjectRef;
use async_trait::async_trait;
use futures::StreamExt;
use kube::{
    api::Api,
    client::Client,
    runtime::{
        controller::{Action, Controller as runtime_controller},
        watcher::Config,
        watcher,
        predicates,
    },
};
use kube::runtime::WatchStreamExt;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;

pub struct BgpRouterController{
    context: Arc<Context>,
    resource: Api<BgpRouter>,
}

impl BgpRouterController{
    pub fn new(client: Client) -> Self{
        let resource = Api::all(client.clone());
        let context = Arc::new(Context{
            client: client.clone(),
        });
        BgpRouterController{context, resource}
    }
    async fn reconcile(g: Arc<BgpRouter>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        Ok(Action::await_change())
    }
    fn error_policy(g: Arc<BgpRouter>, error: &ReconcileError, ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

#[async_trait]
impl Controller for BgpRouterController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<BgpRouter>, ctx: Arc<Context>| {
            async move { BgpRouterController::reconcile(g, ctx).await }
        };
        let error_policy = |g: Arc<BgpRouter>, error: &ReconcileError, ctx: Arc<Context>| {
            BgpRouterController::error_policy(g, error, ctx)
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
