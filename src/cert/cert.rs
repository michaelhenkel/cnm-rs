use rcgen::generate_simple_self_signed;

pub fn get_cert(hostname: &str, ip: &str) -> anyhow::Result<(String, String)>{
    let cert = generate_simple_self_signed(vec![hostname.to_string(), ip.to_string()])?;
    let key = cert.serialize_private_key_pem();
    let cert = cert.serialize_pem()?;
    Ok((key, cert))
}