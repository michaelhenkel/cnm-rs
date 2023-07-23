use std::collections::BTreeMap;
use k8s_openapi::ByteString;
use kube::Client;
use cnm_rs::cert::cert;
use cnm_rs::controllers::controllers;
use k8s_openapi::api::core::v1 as core_v1;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta_v1;
use base64::{Engine as _, engine::{self, general_purpose}, alphabet};
use std::io::Write;
use data_encoding::HEXUPPER;
use ring::error::Unspecified;
use ring::rand::SecureRandom;
use ring::{digest, pbkdf2, rand};
use std::num::NonZeroU32;

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
    //concat the cert and key into a pem file
    let mut pem_file = std::fs::File::create("/etc/certs/tls.pem")?;
    pem_file.write_all(&cert.as_bytes())?;
    pem_file.write_all(&key.as_bytes())?;
    let password = gen_password("admin");



    Ok(())

}

fn gen_password(pwd: &str) -> Result<String, Unspecified>{
    const CREDENTIAL_LEN: usize = digest::SHA512_OUTPUT_LEN;
    let n_iter = NonZeroU32::new(100_000).unwrap();
    let rng = rand::SystemRandom::new();

    let mut salt = [0u8; CREDENTIAL_LEN];
    rng.fill(&mut salt)?;

    
    let mut pbkdf2_hash = [0u8; CREDENTIAL_LEN];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA512,
        n_iter,
        &salt,
        pwd.as_bytes(),
        &mut pbkdf2_hash,
    );

    Ok(HEXUPPER.encode(&pbkdf2_hash))
}

// base_config is a multiline string that contains the base configuration for the device
// it is used to configure the device with a base configuration
// the base configuration is used to configure the device with a base configuration
const BASE_CONFIG: &str = r#"
system {
    host-
    name crpd;
"#;