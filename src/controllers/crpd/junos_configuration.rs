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
    runtime::{
        controller::{Action, Controller as runtime_controller},
        watcher::Config,
    },
};

use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;
use k8s_openapi::api::core::v1 as core_v1;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta_v1;
use std::collections::BTreeMap;
use k8s_openapi::ByteString;

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
            g.meta().namespace.as_ref().unwrap(),
            g.meta().name.as_ref().unwrap(),
            ctx.client.clone())
            .await{
            Ok(res) => {
                match res{
                    Some((bgp_router, _)) => {
                        info!("junos config controller reconciles bgprouter config");
                        if let Some(address) = &bgp_router.spec.v4_address{
                            let mut pod_name = None;
                            bgp_router.meta().owner_references.as_ref().unwrap().iter().for_each(|owner: &meta_v1::OwnerReference| {
                                info!("owner: {:#?}", owner);
                                if owner.kind == "Pod"{
                                    pod_name = Some(owner.name.clone())
                                }
                            });
                            match junos::client::Client::new(
                                address.clone(),
                                pod_name.as_ref().unwrap().clone(),
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
                        if let Some(_status) = bgp_router.status{
                            
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
    fn error_policy(_g: Arc<BgpRouter>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5))
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
        
        let (ca, kp) = match controllers::get::<core_v1::Secret>(self.context.namespace.as_ref().unwrap(), 
        "cnm-ca", self.context.client.clone()).await{
            Ok(ca_secret) => {
                match ca_secret {
                    Some((secret, _)) => {
                        let ca = match secret.data.as_ref().unwrap().get("ca.crt"){
                            Some(ca) => {
                                match std::str::from_utf8(&ca.0){
                                    Ok(ca) => {
                                        ca
                                    },
                                    Err(_e) => {return Err(anyhow::anyhow!("ca.crt is not valid utf8"))}
                                }
                            }
                            None => {return Err(anyhow::anyhow!("ca.crt not found in secret"))}
                        };
                        let kp = match secret.data.as_ref().unwrap().get("kp.crt"){
                            Some(kp) => {
                                match std::str::from_utf8(&kp.0){
                                    Ok(kp) => {
                                        kp
                                    },
                                    Err(_e) => {return Err(anyhow::anyhow!("kp.crt is not valid utf8"))}
                                }
                            }
                            None => {return Err(anyhow::anyhow!("kp.crt not found in secret"))}
                        };
                        (ca.to_string(), kp.to_string())
                    },
                    None => {
                        return Err(anyhow::anyhow!("ca secret not found"));
                    }
                }
            },
            Err(e) => {
                return Err(e.into());
            }
        };

        let ca_cert = match cert::cert::ca_string_to_certificate(ca.clone(), kp.clone(), false){
            Ok(ca_cert) => {
                ca_cert
            },
            Err(e) => {
                return Err(e.into());
            }
        };

        let (key, cert) = match cert::cert::create_sign_private_key(self.context.name.as_ref().unwrap().clone(),
        self.context.address.as_ref().unwrap().clone(),
        ca_cert){
            Ok((key, cert)) => {
                (key, cert)
            },
            Err(e) => {
                return Err(e.into());
            }
        };
        let junos_controller_secret_name = format!("{}-junos-controller", self.context.name.as_ref().unwrap().clone());
        let junos_controller_secret = core_v1::Secret{
            metadata: meta_v1::ObjectMeta{
                name: Some(junos_controller_secret_name.clone()),
                namespace: Some(self.context.namespace.as_ref().unwrap().clone()),
                ..Default::default()
            },
            type_: Some("kubernetes.io/tls".to_string()),
            data: Some(
                BTreeMap::from([
                    ("tls.crt".to_string(), ByteString(cert.as_bytes().to_vec())),
                    ("tls.key".to_string(), ByteString(key.as_bytes().to_vec())),
                    ("ca.crt".to_string(), ByteString(ca.as_bytes().to_vec())),
                ])),
            ..Default::default()
        };

        match controllers::delete::<core_v1::Secret>(self.context.namespace.as_ref().unwrap().clone(), junos_controller_secret_name.clone(), self.context.client.clone()).await{
            Ok(_) => {},
            Err(e) => {
                return Err(e.into());
            }
        }

        match controllers::create_or_update(junos_controller_secret, self.context.client.clone()).await{
            Ok(_) => {},
            Err(e) => {
                return Err(e.into());
            }
        }

        let mut new_context = Context::new(self.context.client.clone());
        new_context.ca = Some(ca.clone());
        new_context.cert = Some(cert.clone());
        new_context.key = Some(key.clone());
        new_context.name = Some(self.context.name.as_ref().unwrap().clone());
        new_context.namespace = Some(self.context.namespace.as_ref().unwrap().clone());
        new_context.address = Some(self.context.address.as_ref().unwrap().clone());


        let mut config = Config::default();
        config.label_selector = Some("
            cnm.juniper.net/bgpRouterManaged=true,
            cnm.juniper.net/instanceType=Crpd
        ".to_string());
        runtime_controller::new(self.resource.clone(), config)
            .run(reconcile, error_policy, Arc::new(new_context))
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
