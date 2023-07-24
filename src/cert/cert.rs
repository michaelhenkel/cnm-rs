use rcgen::{generate_simple_self_signed, KeyPair, DnType, DnValue,CertificateParams, DistinguishedName,Certificate, SanType, IsCa, BasicConstraints, CertificateSigningRequest};
use openssl::asn1::{Asn1Time, Asn1IntegerRef};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::asn1::Asn1Integer;
use openssl::x509::{X509, X509NameBuilder, X509Name};
use openssl::x509::extension::{AuthorityKeyIdentifier, ExtendedKeyUsage, KeyUsage, SubjectAlternativeName};
use tracing::{info, warn};
use std::f32::consts::E;
use std::io::Write;

pub fn get_cert(hostname: &str, ip: &str) -> anyhow::Result<(String, String)>{
    let cert = generate_simple_self_signed(vec![hostname.to_string(), ip.to_string()])?;
    
    let key = cert.serialize_private_key_pem();
    let cert = cert.serialize_pem()?;
    Ok((key, cert))
}

pub fn create_ca_key_cert(common_name: String) -> anyhow::Result<(String, Certificate)> {
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(DnType::CommonName, DnValue::PrintableString(common_name));
    let mut certificate_params = CertificateParams::default();
    certificate_params.distinguished_name = DistinguishedName::new();
    certificate_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let cert = Certificate::from_params(certificate_params)?;
    let ca_cert = cert.serialize_pem()?;
    Ok((ca_cert, cert))
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