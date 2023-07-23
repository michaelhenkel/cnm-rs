use std::collections::BTreeMap;
use k8s_openapi::ByteString;
use kube::Client;
use cnm_rs::cert::cert;
use cnm_rs::controllers::controllers;
use k8s_openapi::api::core::v1 as core_v1;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta_v1;
use base64::{Engine as _, engine::{self, general_purpose}, alphabet};
use std::io::Write;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let pod_ip = match std::env::var("POD_IP"){
        Ok(pod_ip) => { pod_ip }
        Err(e) => { return Err(e.into())}
    };
    let pod_name = match std::env::var("POD_NAME"){
        Ok(pod_name) => { pod_name }
        Err(e) => { return Err(e.into())}
    };
    let pod_namespace = match std::env::var("POD_NAMESPACE"){
        Ok(pod_namespace) => { pod_namespace }
        Err(e) => { return Err(e.into())}
    };
    let (key, cert) = cert::get_cert(pod_name.as_str(), pod_ip.as_str())?;
    let key_bs = ByteString(general_purpose::STANDARD.encode(&key).as_bytes().to_vec());
    let cert_bs = ByteString(general_purpose::STANDARD.encode(&cert).as_bytes().to_vec());
    let secret = core_v1::Secret{
        metadata: meta_v1::ObjectMeta{
            name: Some(pod_name),
            namespace: Some(pod_namespace),
            ..Default::default()
        },
        data: Some(BTreeMap::from([("tls.crt".to_string(), cert_bs), ("tls.key".to_string(), key_bs)])),
        ..Default::default()
    };
    let client = Client::try_default().await?;
    controllers::create_or_update(secret, client).await?;
    //write the cert to a file
    let mut cert_file = std::fs::File::create("/etc/certs/tls.crt")?;
    cert_file.write_all(&cert.as_bytes())?;
    let mut key_file = std::fs::File::create("/etc/certs/tls.key")?;
    key_file.write_all(&key.as_bytes())?;

    Ok(())

}