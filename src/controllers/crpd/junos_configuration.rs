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
                    Some((bgp_router, _)) => {
                        info!("junos config controller reconciles bgprouter config");
                        let (key, pem) = match controllers::get::<core_v1::Secret>(bgp_router.meta().namespace.as_ref().unwrap().to_string(), 
                        bgp_router.meta().name.as_ref().unwrap().to_string(), ctx.client.clone()).await{
                            Ok(secret) => {
                                if let Some((secret, _)) = secret{
                                    let key = match secret.data.as_ref().unwrap().get("tls.key"){
                                        Some(key) => {
                                            match std::str::from_utf8(&key.0){
                                                Ok(key) => {
                                                    info!("key {:#?}", key);
                                                    key
                                                },
                                                Err(e) => {return Err(ReconcileError(anyhow::anyhow!("tls.key is not valid utf8")))}
                                            }
                                            
                                        }
                                        None => {return Err(ReconcileError(anyhow::anyhow!("tls.key not found in secret")))}
                                    };
                                    let cert = match secret.data.as_ref().unwrap().get("tls.crt"){
                                        Some(cert) => {
                                            match std::str::from_utf8(&cert.0){
                                                Ok(cert) => {
                                                    info!("cert {:#?}", cert);
                                                    cert
                                                },
                                                Err(e) => {return Err(ReconcileError(anyhow::anyhow!("tls.crt is not valid utf8")))}
                                            }
                                        }
                                        None => {return Err(ReconcileError(anyhow::anyhow!("tls.crt not found in secret")))}
                                    };
                                    (key.to_string(), format!("{}\n{}", key, cert))
                                } else {
                                    return Err(ReconcileError(anyhow::anyhow!("secret not found")))
                                }
                               
                            },
                            Err(e) => {return Err(e)}
                        };
                        if let Some(address) = bgp_router.spec.address{
                            match junos::client::Client::new(address, key, pem).await{
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
