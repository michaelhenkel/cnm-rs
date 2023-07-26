use std::collections::BTreeMap;
use k8s_openapi::ByteString;
use kube::Client;
use cnm_rs::cert::cert;
use cnm_rs::controllers::controllers;
use k8s_openapi::api::core::v1 as core_v1;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta_v1;
use base64::{Engine as _, engine::{self, general_purpose}, alphabet};
use kube_runtime::controller;
use std::io::Write;
use data_encoding::HEXUPPER;
use ring::error::Unspecified;
use ring::rand::SecureRandom;
use ring::{digest, pbkdf2, rand};
use std::num::NonZeroU32;
use std::io::Read;
use pwhash::unix;
use pwhash::bcrypt;


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

    let client = Client::try_default().await?;

    let (ca, kp) = match controllers::get::<core_v1::Secret>(pod_namespace.clone(), 
    "cnm-ca".to_string(), client.clone()).await{
        Ok(ca_secret) => {
            match ca_secret {
                Some((secret, _)) => {
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
                    return Err(anyhow::anyhow!("ca secret not found"));
                }
            }
        },
        Err(e) => { return Err(e.into())},
    };

    let ca_cert = match cert::ca_string_to_certificate(ca.clone(), kp.clone(), false){
        Ok(ca_cert) => {
            ca_cert
        },
        Err(e) => {
            return Err(e.into());
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

    controllers::delete::<core_v1::Secret>(pod_namespace.clone(), pod_name.clone(), client.clone()).await?;

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
                ("ca.crt".to_string(), ByteString(ca.as_bytes().to_vec())),
            ])),
        ..Default::default()
    };
    
    controllers::create_or_update(secret, client).await?;
    //write the cert to a file
    let mut cert_file = std::fs::File::create("/etc/certs/tls.crt")?;
    cert_file.write_all(&signed_cert.as_bytes())?;
    let mut key_file = std::fs::File::create("/etc/certs/tls.key")?;
    key_file.write_all(&private_key.as_bytes())?;
    //concat the cert and key into a pem file
    let mut pem_file = std::fs::File::create("/etc/certs/tls.pem")?;
    pem_file.write_all(&private_key.as_bytes())?;
    pem_file.write_all(&signed_cert.as_bytes())?;
    
    let single_line_cert = read_file("/etc/certs/tls.pem")?;
    if let Ok(passwpord) = gen_password("Juniper123") {
        write_config(&generate_config(&passwpord, &single_line_cert))?;
        gzip_config()?;
    } else {
        return Err(anyhow::anyhow!("Failed to generate password"));
    }
    Ok(())

}

fn gen_password(pwd: &str) -> anyhow::Result<String>{
    let h = bcrypt::hash(pwd).unwrap();
    Ok(unix::crypt(pwd, h.as_str())?)
}

// base_config is a multiline string that contains the base configuration for the device
// it is used to configure the device with a base configuration
// the base configuration is used to configure the device with a base configuration
const BASE_CONFIG: &str = r#"
system {
    root-authentication {
        encrypted-password "PASSWORD";
    }
    services {
        ssh {
            root-login allow;
            port 24;
        }
        extension-service {
            request-response {
                grpc {
                    ssl {
                        port 50052;
                        local-certificate grpc;
                    }
                    skip-authentication;
                }  
            }
            traceoptions {
                file jsd;
                flag all;
            }
        }
        netconf {
            ssh;
        }
    }
}
security {
    certificates {
        local {
            grpc {
                "KEY";
            }
        }
    }
}
"#;

// read a file and put all lines into a single line separated by \n
fn read_file(path: &str) -> Result<String, std::io::Error> {
    let mut file = std::fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents.replace("\n", "\\n"))
}

// generate_config generates a configuration for the device. It replaces PASSWORD and KEY with the
// password and key that are passed in.
fn generate_config(password: &str, key: &str) -> String {
    BASE_CONFIG
        .replace("PASSWORD", password)
        .replace("KEY", key)
}

// write_config writes the configuration to the device
fn write_config(config: &str) -> Result<(), std::io::Error> {
    let mut file = std::fs::File::create("/tmp/juniper.conf")?;
    file.write_all(config.as_bytes())?;
    Ok(())
}

// gzip the configuration
fn gzip_config() -> Result<(), std::io::Error> {
    let mut file = std::fs::File::open("/tmp/juniper.conf")?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&contents)?;
    let compressed_contents = encoder.finish()?;
    let mut file = std::fs::File::create("/config/juniper.conf.gz")?;
    file.write_all(&compressed_contents)?;
    Ok(())
}
