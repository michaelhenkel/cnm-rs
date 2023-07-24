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


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long, default_value_t = "bla")]
    name: String,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
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

    let client = Client::try_default().await?;

    let (ca_cert_string, ca_cert) = match cert::create_ca_key_cert(pod_name.clone()){
        Ok((ca_cert_string, ca_cert)) => {
            (ca_cert_string, ca_cert)
        },
        Err(e) => {
            return Err(e);
        }
    };

    let (private_key, signed_cert) = match cert::create_sign_private_key(pod_name.clone(), pod_ip.clone(), ca_cert){
        Ok((private_key, signed_cert)) => {
            (private_key, signed_cert)
        },
        Err(e) => {
            return Err(e);
        }
    };

    let secret = core_v1::Secret{
        metadata: meta_v1::ObjectMeta{
            name: Some(pod_name),
            namespace: Some(pod_namespace),
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
    let client = Client::try_default().await?;
    controllers::create_or_update(secret, client).await?;

    let mut join_handlers = Vec::new();

    let adm = admission::admission::AdmissionController::new("192.168.105.4".to_string(),"cnm-admission-controller".to_string(), client.clone());
    
    join_handlers.push(tokio::spawn(async move {
        adm.admission().await
    }));



    let resource_list: Vec<Box<dyn resources::resources::Resource>> = vec![

        Box::new(resources::crpd::crpd::CrpdResource::new(client.clone())),
        Box::new(resources::bgp_router::BgpRouterResource::new(client.clone())),
        Box::new(resources::bgp_router_group::BgpRouterGroupResource::new(client.clone())),

    ];
    resources::resources::init_resources(resource_list).await?;

    let controller_list: Vec<Box<dyn controllers::Controller>> = vec![

        Box::new(CrpdController::new(client.clone())),
        Box::new(BgpRouterController::new(client.clone())),
        Box::new(BgpRouterGroupController::new(client.clone())),
        Box::new(JunosConfigurationController::new(client.clone())),

    ];
    

    join_handlers.push(
        tokio::spawn(async move {
            controllers::init_controllers(controller_list).await
        })
    );

    futures::future::join_all(join_handlers).await;

    Ok(())
}
