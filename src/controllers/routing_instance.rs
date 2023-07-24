use crate::controllers::controllers::{Controller, Context, ReconcileError};
use crate::resources::routing_instance::RoutingInstance;
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

pub struct RoutingInstanceController{
    context: Arc<Context>,
    resource: Api<RoutingInstance>,
}

impl RoutingInstanceController{
    pub fn new(client: Client) -> Self{
        let resource = Api::all(client.clone());
        let context = Arc::new(Context::new(client.clone()));
        RoutingInstanceController{context, resource}
    }
    async fn reconcile(g: Arc<RoutingInstance>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        Ok(Action::await_change())
    }
    fn error_policy(g: Arc<RoutingInstance>, error: &ReconcileError, ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

#[async_trait]
impl Controller for RoutingInstanceController{
    async fn run(&self) -> anyhow::Result<()>{
        let reconcile = |g: Arc<RoutingInstance>, ctx: Arc<Context>| {
            async move { RoutingInstanceController::reconcile(g, ctx).await }
        };
        let error_policy = |g: Arc<RoutingInstance>, error: &ReconcileError, ctx: Arc<Context>| {
            RoutingInstanceController::error_policy(g, error, ctx)
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
