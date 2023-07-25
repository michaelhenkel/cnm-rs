use cnm_rs::controllers::controllers::Context;
use cnm_rs::resources;
use cnm_rs::controllers::{
    crpd::crpd::CrpdController,
    bgp_router::BgpRouterController,
    crpd::bgp_router_group::BgpRouterGroupController,
    crpd::junos_configuration::JunosConfigurationController,
    controllers,
};
use cnm_rs::admission;
use cnm_rs::cert::cert;
use kube::Client;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta_v1;
use k8s_openapi::api::core::v1 as core_v1;
use std::collections::BTreeMap;
use k8s_openapi::ByteString;
use clap::Parser;
use std::sync::Arc;
use tracing::info;



#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    name: Option<String>,

    #[arg(short, long)]
    namespace: Option<String>,

    #[arg(short, long)]
    address: Option<String>,
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
    .event_format(
        tracing_subscriber::fmt::format()
            .with_file(true)
            .with_line_number(true)
    )
    .init();

    let args = Args::parse();

    let name = if let Some(name) = args.name{
        name
    } else {
        match std::env::var("POD_NAME"){
            Ok(pod_name) => { pod_name }
            Err(e) => { return Err(e.into())}
        } 
    };

    let address = if let Some(address) = args.address{
        address
    } else {
        match std::env::var("POD_IP"){
            Ok(pod_ip) => { pod_ip }
            Err(e) => { return Err(e.into())}
        } 
    };

    let namespace = if let Some(namespace) = args.namespace{
        namespace
    } else {
        match std::env::var("POD_NAMESPACE"){
            Ok(namespace) => { namespace }
            Err(e) => { return Err(e.into())}
        } 
    };

    let client = Client::try_default().await?;

    let secret = match controllers::get::<core_v1::Secret>(namespace.clone(), name.clone(), client.clone()).await{
        Ok(secret) => { secret },
        Err(e) => { return Err(e.into())},
    };

    let (cert, key, ca) = match secret{
        Some((secret, _api)) => {
            let key = match secret.data.as_ref().unwrap().get("tls.key"){
                Some(key) => {
                    match std::str::from_utf8(&key.0){
                        Ok(key) => {
                            key
                        },
                        Err(e) => {return Err(anyhow::anyhow!("tls.key is not valid utf8"))}
                    }
                    
                }
                None => {return Err(anyhow::anyhow!("tls.key not found in secret"))}
            };
            let cert = match secret.data.as_ref().unwrap().get("tls.crt"){
                Some(cert) => {
                    match std::str::from_utf8(&cert.0){
                        Ok(cert) => {
                            cert
                        },
                        Err(e) => {return Err(anyhow::anyhow!("tls.crt is not valid utf8"))}
                    }
                }
                None => {return Err(anyhow::anyhow!("tls.crt not found in secret"))}
            };
            let ca = match secret.data.as_ref().unwrap().get("ca.crt"){
                Some(ca) => {
                    match std::str::from_utf8(&ca.0){
                        Ok(ca) => {
                            ca
                        },
                        Err(e) => {return Err(anyhow::anyhow!("ca.crt is not valid utf8"))}
                    }
                }
                None => {return Err(anyhow::anyhow!("ca.crt not found in secret"))}
            };
            (cert.to_string(), key.to_string(), ca.to_string())
        },
        None => {
            let (ca_cert_string, ca_cert) = match cert::create_ca_key_cert(name.clone()){
                Ok((ca_cert_string, ca_cert)) => {
                    (ca_cert_string, ca_cert)
                },
                Err(e) => {
                    return Err(e);
                }
            };
        
            let (private_key, signed_cert) = match cert::create_sign_private_key(name.clone(), address.clone(), ca_cert){
                Ok((private_key, signed_cert)) => {
                    (private_key, signed_cert)
                },
                Err(e) => {
                    return Err(e);
                }
            };
        
            let secret = core_v1::Secret{
                metadata: meta_v1::ObjectMeta{
                    name: Some(name.clone()),
                    namespace: Some(namespace.clone()),
                    ..Default::default()
                },
                type_: Some("kubernetes.io/tls".to_string()),
                data: Some(
                    BTreeMap::from([
                        ("tls.crt".to_string(), ByteString(signed_cert.as_bytes().to_vec())),
                        ("tls.key".to_string(), ByteString(private_key.as_bytes().to_vec())),
                        ("ca.crt".to_string(), ByteString(ca_cert_string.as_bytes().to_vec())),
                    ])),
                ..Default::default()
            };
            controllers::create_or_update(secret, client.clone()).await?;
            (signed_cert, private_key, ca_cert_string)
        },
    };

    let mut join_handlers = Vec::new();

    let adm = admission::admission::AdmissionController::new(address.clone(),"cnm-admission-controller".to_string(), client.clone());
    
    join_handlers.push(tokio::spawn(async move {
        adm.admission().await
    }));

    let mut ctx = Context::new(client.clone());
    ctx.address = Some(address.clone());
    ctx.name = Some(name.clone());
    ctx.namespace = Some(namespace.clone());
    ctx.cert = Some(cert.clone());
    ctx.key = Some(key.clone());
    ctx.ca = Some(ca.clone());

    let ctx = Arc::new(ctx);

    let resource_list: Vec<Box<dyn resources::resources::Resource>> = vec![
        Box::new(resources::crpd::crpd::CrpdResource::new(client.clone())),
        Box::new(resources::bgp_router::BgpRouterResource::new(client.clone())),
        Box::new(resources::bgp_router_group::BgpRouterGroupResource::new(client.clone())),
    ];
    resources::resources::init_resources(resource_list).await?;

    let controller_list: Vec<Box<dyn controllers::Controller>> = vec![
        Box::new(CrpdController::new(ctx.clone())),
        Box::new(BgpRouterController::new(ctx.clone())),
        Box::new(BgpRouterGroupController::new(ctx.clone())),
        Box::new(JunosConfigurationController::new(ctx.clone())),
    ];
    
    join_handlers.push(
        tokio::spawn(async move {
            controllers::init_controllers(controller_list).await
        })
    );

    futures::future::join_all(join_handlers).await;

    Ok(())
}
