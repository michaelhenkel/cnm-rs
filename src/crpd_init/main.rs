use std::{collections::{BTreeMap, HashMap}, any};
use kube::Client;
use cnm_rs::{
    cert::cert,
    controllers::{controllers, crpd::junos::family}, 
    resources::crpd::crpd::{
        Crpd,
        Instance,
        Interface
    },
    resources::interface,
};
use k8s_openapi::{
    api::core::v1 as core_v1,
    apimachinery::pkg::apis::meta::v1 as meta_v1,
    ByteString
};
use tracing::info;
use std::io::{Write, Read};
use pwhash::{unix, bcrypt};
use interfaces;
use std::sync::Arc;


#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let pod_ip = match std::env::var("POD_IP"){
        Ok(pod_ip) => pod_ip,
        Err(e) => return Err(e.into())
    };
    let pod_name = match std::env::var("POD_NAME"){
        Ok(pod_name) => pod_name,
        Err(e) => return Err(e.into())
    };
    let pod_namespace = match std::env::var("POD_NAMESPACE"){
        Ok(pod_namespace) => pod_namespace,
        Err(e) => return Err(e.into())
    };
    let pod_uuid = match std::env::var("POD_UUID"){
        Ok(pod_uuid) => pod_uuid,
        Err(e) => return Err(e.into())
    };
    let crpd_group = match std::env::var("CRPD_GROUP"){
        Ok(crpd_group) => crpd_group,
        Err(e) => return Err(e.into())
    };

    let client = Client::try_default().await?;

    let (ca, kp) = match controllers::get::<core_v1::Secret>(
        &pod_namespace, 
        "cnm-ca",
        client.clone()).await{
        Ok(ca_secret) => {
            match ca_secret {
                Some((secret, _)) => {
                    let ca = match secret.data.as_ref().unwrap().get("ca.crt"){
                        Some(ca) => {
                            match std::str::from_utf8(&ca.0){
                                Ok(ca) => ca,
                                Err(e) => return Err(e.into())
                            }
                        }
                        None =>  return Err(anyhow::anyhow!("ca.crt not found in secret"))
                    };
                    let kp = match secret.data.as_ref().unwrap().get("kp.crt"){
                        Some(kp) => {
                            match std::str::from_utf8(&kp.0){
                                Ok(kp) => {kp},
                                Err(e) => {return Err(e.into())}
                            }
                        }
                        None => return Err(anyhow::anyhow!("kp.crt not found in secret"))
                    };
                    (ca.to_string(), kp.to_string())
                },
                None => return Err(anyhow::anyhow!("ca secret not found"))
            }
        },
        Err(e) => return Err(e.into()),
    };

    let ca_cert = match cert::ca_string_to_certificate(ca.clone(), kp.clone(), false){
        Ok(ca_cert) =>  ca_cert,
        Err(e) => return Err(e.into())
    };
    
    let (private_key, signed_cert) = match cert::create_sign_private_key(pod_name.clone(), pod_ip.clone(), ca_cert){
        Ok(res) => res,
        Err(e) => return Err(e)
    };

    controllers::delete::<core_v1::Secret>(pod_namespace.clone(), pod_name.clone(), client.clone()).await?;

    let secret = core_v1::Secret{
        metadata: meta_v1::ObjectMeta{
            name: Some(pod_name.clone()),
            namespace: Some(pod_namespace.clone()),
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
    
    controllers::create_or_update(secret, client.clone()).await?;
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



    let mut instance_interfaces = BTreeMap::new();

    let interface_list = interfaces::Interface::get_all()?;
    for intf in interface_list{
        if !intf.is_loopback(){
            let mut local_interface = Interface{
                ..Default::default()
            };
            let mac = intf.hardware_addr()?.to_string();
            if mac != "00:00:00:00:00:00"{
                local_interface.mac = mac
            }
            let mut found_v4 = false;
            let mut found_v6 = false;
            for addr in &intf.addresses{
                if let Some(intf_addr) = &addr.addr{
                    match addr.kind{
                        interfaces::Kind::Ipv4 => {
                            if !found_v4{
                                let prefix: std::net::Ipv4Addr = intf_addr.ip().to_string().parse()?;
                                let mask: std::net::Ipv4Addr = addr.mask.unwrap().ip().to_string().parse()?;
                                let cidr = get_cidr(mask.octets());
                                local_interface.v4_address = Some(format!("{}/{}", prefix.to_string(), cidr));
                                found_v4 = true;
                            }
                        },
                        interfaces::Kind::Ipv6 => {
                            if !found_v6{
                                let prefix: std::net::Ipv6Addr = intf_addr.ip().to_string().parse()?;
                                let v6_int = as_u128_be(&prefix.octets());
                                let s = v6_int >> 112;
                                if s as u16 != 0xfe80 {
                                    let mask: std::net::Ipv6Addr = addr.mask.unwrap().ip().to_string().parse()?;
                                    let cidr = get_cidr(mask.octets());
                                    local_interface.v6_address = Some(format!("{}/{}", prefix.to_string(), cidr));
                                }
                                found_v6 = true;
                            }
                        },
                        _ => {}
                    }
                }

            }
            if (local_interface.v4_address.is_some() || local_interface.v6_address.is_some()) && !local_interface.mac.is_empty(){
                instance_interfaces.insert(intf.name.clone(), local_interface);
            }
            
        }

    }
    let instance = Instance{
        //name: pod_name.clone(),
        uuid: pod_uuid.clone(),
        interfaces: instance_interfaces,
    };


    let mut crpd = match controllers::get::<Crpd>(&pod_namespace, &crpd_group, client.clone()).await{
        Ok(crpd) => {
            match crpd {
                Some((crpd, _)) => {
                    crpd
                },
                None => return Err(anyhow::anyhow!("crpd not found"))
            }
        },
        Err(e) => return Err(e.into())
    };

    if let Some(status) = crpd.status.as_mut(){
        match status.instances.as_mut(){
            Some(instances) => {
                instances.insert(pod_name.clone(), instance.clone());
            },
            None => {
                let mut instances = BTreeMap::new();
                instances.insert(pod_name.clone(), instance.clone());
                status.instances = Some(instances);
            }
        }
    }

    info!("status: {:#?}", crpd.status);

    if let Err(e) = controllers::update_status(crpd.clone(), client.clone()).await{
        return Err(e.into());
    }

    if crpd.spec.setup_interfaces{
        for (intf_name, intf) in &instance.interfaces{
            let interface = interface::Interface{
                metadata: meta_v1::ObjectMeta {
                    name: Some(format!("{}-{}", pod_name.clone(), intf_name)),
                    namespace: Some(pod_namespace.clone()),
                    labels: Some(
                        BTreeMap::from([("cnm.juniper.net/crpdGroup".to_string(),crpd_group.clone())])
                    ),
                    ..Default::default()
                },
                spec: interface::InterfaceSpec {
                    parent: core_v1::LocalObjectReference { name: Some(pod_name.clone()) },
                    mac: intf.mac.clone(),
                    mtu: 8900,
                    families: {
                        let mut family_list = Vec::new();
                        if let Some(v4_ip) = &intf.v4_address{
                            let interface_inet = interface::InterfaceInet{
                                address: v4_ip.clone()
                            };
                            let fam = interface::InterfaceFamily::Inet(interface_inet);
                            family_list.push(fam);
                        }
                        if let Some(v6_ip) = &intf.v6_address{
                            let interface_inet6 = interface::InterfaceInet6{
                                address: v6_ip.clone()
                            };
                            let fam = interface::InterfaceFamily::Inet6(interface_inet6);
                            family_list.push(fam);
                        }
                        family_list
                    }
                    
                },
                status: None
            };
            if let Err(e) = controllers::create(Arc::new(interface), client.clone()).await{
                return Err(e.into());
            }
        }
    }

    // read linux interface configuration from the operating system



    Ok(())

}

pub fn as_u128_be(array: &[u8; 16]) -> u128 {
    ((array[0] as u128) << 120) +
    ((array[1] as u128) << 112) +
    ((array[2] as u128) << 104) +
    ((array[3] as u128) << 96) +
    ((array[4] as u128) << 88) +
    ((array[5] as u128) << 80) +
    ((array[6] as u128) << 72) +
    ((array[7] as u128) << 64) +
    ((array[8] as u128) << 56) +
    ((array[9] as u128) << 48) +
    ((array[10] as u128) << 40) +
    ((array[11] as u128) << 32) +
    ((array[12] as u128) << 24) +
    ((array[13] as u128) << 16) +
    ((array[14] as u128) <<  8) +
    ((array[15] as u128) <<  0)
}

fn get_cidr<const N: usize>(octets: [u8;N]) -> u8  {
    let mut cidr: u8 = 0;
    for octet in octets {
        let mut bits = octet;
        while bits > 0 {
            cidr += 1;
            bits <<= 1;
        }
    }
    cidr
}

fn gen_password(pwd: &str) -> anyhow::Result<String>{
    let h = bcrypt::hash(pwd).unwrap();
    Ok(unix::crypt(pwd, h.as_str())?)
}

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
