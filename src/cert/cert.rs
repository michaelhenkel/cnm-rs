use rcgen::{generate_simple_self_signed, KeyPair, DnType, DnValue,CertificateParams, DistinguishedName,Certificate, IsCa, BasicConstraints, CertificateSigningRequest, PKCS_RSA_SHA256};











pub fn create_ca_key_cert(common_name: String) -> anyhow::Result<(String, String)> {
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(DnType::CommonName, DnValue::PrintableString(common_name));
    let mut certificate_params = CertificateParams::default();
    certificate_params.distinguished_name = DistinguishedName::new();
    certificate_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let cert = Certificate::from_params(certificate_params)?;
    let ca_string = cert.serialize_pem()?;
    Ok((ca_string, cert.get_key_pair().serialize_pem()))
}

pub fn ca_string_to_certificate(ca_string: String, kp_string: String, rsa: bool) -> anyhow::Result<Certificate>{
    let kp = if rsa{
        //let k = KeyPair::from_pem_and_sign_algo(x.as_str(), &PKCS_RSA_SHA256)?;
        match KeyPair::from_pem_and_sign_algo(&kp_string, &PKCS_RSA_SHA256){
            Ok(kp) => {
                kp
            },
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to create key pair from pem: {}", e));
            }
        }
    }else {
        match KeyPair::from_pem(&kp_string){
            Ok(kp) => {
                kp
            },
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to create key pair from pem: {}", e));
            }
        }
    };
    let cert_param = match CertificateParams::from_ca_cert_pem(&ca_string, kp){
        Ok(cert_param) => {
            cert_param
        },
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to create certificate params from ca cert pem: {}", e));
        }
    };
    let cert = match Certificate::from_params(cert_param){
        Ok(cert) => {
            cert
        },
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to create certificate from params: {}", e));
        }
    };
    Ok(cert)
}

pub fn create_sign_private_key(common_name: String, address: String, ca_certificate: Certificate) -> anyhow::Result<(String, String)> {
    let cert = match generate_simple_self_signed(vec![common_name, address]){
        Ok(cert) => {
            cert
        },
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to create certificate from params: {}", e));
        }
    };


    let csr_pem = match cert.serialize_request_pem(){
        Ok(csr) => {
            csr
        },
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to create certificate signing request: {}", e));
        }
    };

    let csr = match CertificateSigningRequest::from_pem(&csr_pem){
        Ok(csr) => {
            csr
        },
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to create certificate signing request from pem: {}", e));
        }
    };

    let cert_pem = match csr.serialize_pem_with_signer(&ca_certificate){
        Ok(cert_pem) => {
            cert_pem
        },
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to sign certificate: {}", e));
        }
    }; 
    //self.private_signed_cert = Some(signed_cert);
    Ok((cert.serialize_private_key_pem(), cert_pem))
}