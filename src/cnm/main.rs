use cnm_rs::controllers::controllers::Context;
use cnm_rs::resources;
use cnm_rs::controllers::{
    crpd::crpd::CrpdController,
    bgp_router::BgpRouterController,
    crpd::bgp_router_group::BgpRouterGroupController,
    crpd::junos_configuration::JunosConfigurationController,
    routing_instance::RoutingInstanceController,
    pool::PoolController,
    ip_address::IpAddressController,
    crpd::interface::InterfaceController,
    crpd::vrrp::VrrpController,
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
    #[arg(short, long, default_value = "cnm")]
    name: Option<String>,

    #[arg(long, default_value = "default")]
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

    let secret = match controllers::get::<core_v1::Secret>(&namespace, "cnm-ca", client.clone()).await{
        Ok(secret) => { secret },
        Err(e) => { return Err(e.into())},
    };

    let (ca, kp) = match secret{
        Some((secret, _api)) => {
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
            let kp = match secret.data.as_ref().unwrap().get("kp.crt"){
                Some(kp) => {
                    match std::str::from_utf8(&kp.0){
                        Ok(kp) => {
                            kp
                        },
                        Err(e) => {return Err(anyhow::anyhow!("kp.crt is not valid utf8"))}
                    }
                }
                None => {return Err(anyhow::anyhow!("kp.crt not found in secret"))}
            };
            (ca.to_string(), kp.to_string())
        },
        None => {
            let (ca, kp)  = match cert::create_ca_key_cert(name.clone()){
                Ok(ca_cert_string) => {
                    ca_cert_string
                },
                Err(e) => {
                    return Err(e);
                }
            };
            let secret = core_v1::Secret{
                metadata: meta_v1::ObjectMeta{
                    name: Some("cnm-ca".to_string()),
                    namespace: Some(namespace.clone()),
                    ..Default::default()
                },
                //type_: Some("kubernetes.io/tls".to_string()),
                data: Some(
                    BTreeMap::from([
                        ("ca.crt".to_string(), ByteString(ca.as_bytes().to_vec())),
                        ("kp.crt".to_string(), ByteString(kp.as_bytes().to_vec())),
                    ])),
                ..Default::default()
            };
            controllers::create_or_update(secret, client.clone()).await?;
            (ca, kp)
        },
    };

    let ca_test = match cert::ca_string_to_certificate(ca.clone(), kp.clone(), false){
        Ok(ca_cert) => {
            ca_cert
        },
        Err(e) => {
            info!("Failed to create ca cert: {}", e);
            return Err(e.into());
        }
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
    ctx.ca = Some(ca.clone());

    let ctx = Arc::new(ctx);

    let resource_list: Vec<Box<dyn resources::resources::Resource>> = vec![
        Box::new(resources::crpd::crpd::CrpdResource::new(client.clone())),
        Box::new(resources::bgp_router::BgpRouterResource::new(client.clone())),
        Box::new(resources::bgp_router_group::BgpRouterGroupResource::new(client.clone())),
        Box::new(resources::routing_instance::RoutingInstanceResource::new(client.clone())),
        Box::new(resources::pool::PoolResource::new(client.clone())),
        Box::new(resources::ip_address::IpAddressResource::new(client.clone())),
        Box::new(resources::interface::InterfaceResource::new(client.clone())),
        Box::new(resources::vrrp::VrrpResource::new(client.clone())),
    ];
    resources::resources::init_resources(resource_list).await?;

    let controller_list: Vec<Box<dyn controllers::Controller>> = vec![
        Box::new(CrpdController::new(ctx.clone())),
        Box::new(BgpRouterController::new(ctx.clone())),
        Box::new(BgpRouterGroupController::new(ctx.clone())),
        Box::new(JunosConfigurationController::new(ctx.clone())),
        Box::new(RoutingInstanceController::new(ctx.clone())),
        Box::new(IpAddressController::new(ctx.clone())),
        Box::new(PoolController::new(ctx.clone())),
        Box::new(InterfaceController::new(ctx.clone())),
        Box::new(VrrpController::new(ctx.clone())),
    ];
    
    join_handlers.push(
        tokio::spawn(async move {
            controllers::init_controllers(controller_list).await
        })
    );

    futures::future::join_all(join_handlers).await;

    Ok(())
}
