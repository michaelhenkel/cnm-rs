use rcgen::{generate_simple_self_signed, KeyPair, DnType, DnValue,CertificateParams, DistinguishedName,Certificate, SanType, IsCa, BasicConstraints, CertificateSigningRequest};
use openssl::asn1::{Asn1Time, Asn1IntegerRef};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::asn1::Asn1Integer;
use openssl::x509::{X509, X509NameBuilder, X509Name};
use openssl::x509::extension::{AuthorityKeyIdentifier, ExtendedKeyUsage, KeyUsage, SubjectAlternativeName};
use std::io::Write;

pub fn get_cert(hostname: &str, ip: &str) -> anyhow::Result<(String, String)>{
    let cert = generate_simple_self_signed(vec![hostname.to_string(), ip.to_string()])?;
    
    let key = cert.serialize_private_key_pem();
    let cert = cert.serialize_pem()?;
    Ok((key, cert))
}

pub fn get_cert2(hostname: &str, ip: &str) -> anyhow::Result<()>{
    let mut certificate_params = CertificateParams::default();
    let hostname_san = SanType::DnsName(hostname.to_string());
    let ip_san = SanType::IpAddress(ip.parse()?);
    certificate_params.subject_alt_names = vec![hostname_san, ip_san];
    certificate_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let mut cert = Certificate::from_params(certificate_params)?;
    Ok(())
}

pub struct Cert{
    pub ca_key: Option<String>,
    pub ca_cert: Option<String>,
    pub address: String,
    pub dns: String,
    pub private_key: Option<String>,
    pub private_cert: Option<String>,
    pub private_signed_cert: Option<String>,
}

impl Cert{
    pub fn new(address: String, dns: String) -> Self{
        Self {
            ca_key: None,
            ca_cert: None,
            address,
            dns,
            private_key: None,
            private_cert: None,
            private_signed_cert: None,
        }
    }
    pub fn create_ca_key_cert(&mut self) -> anyhow::Result<()> {
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, DnValue::PrintableString(self.dns.clone()));
        let mut certificate_params = CertificateParams::default();
        certificate_params.distinguished_name = DistinguishedName::new();
        certificate_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let cert = Certificate::from_params(certificate_params)?;
        let key = cert.serialize_private_key_pem();
        let cert = cert.serialize_pem()?;
        self.ca_key = Some(key);
        self.ca_cert = Some(cert);
        Ok(())
    }
    pub fn create_sign_private_key(&mut self) -> anyhow::Result<()> {
        let kp = KeyPair::from_pem(self.ca_cert.as_ref().unwrap().as_str())?;
        let ca_certificate_params = CertificateParams::from_ca_cert_pem(self.ca_cert.as_ref().unwrap().as_str(), kp)?;
        let ca_cert = Certificate::from_params(ca_certificate_params)?;
        let cert = generate_simple_self_signed(vec![self.dns.to_string(), self.address.to_string()])?;
        let key = cert.serialize_private_key_pem();
        let cert = cert.serialize_pem()?;

        let cert_signing_request = CertificateSigningRequest::from_pem(cert.as_str())?;
        let signed_cert = cert_signing_request.serialize_pem_with_signer(&ca_cert)?;
        self.private_key = Some(key);
        self.private_cert = Some(cert);
        self.private_signed_cert = Some(signed_cert);
        Ok(())
    }
}


pub struct CertificateX{
    pub ca_key: Vec<u8>,
    pub ca_cert: Vec<u8>,
    pub address: String,
    pub dns: Option<String>,
    pub private_key: Vec<u8>,
    pub proxy_cert: Vec<u8>,
}

impl CertificateX{
    pub fn new(address: String, dns: Option<String>) -> Self{
        Self { 
            ca_key: Vec::new(),
            ca_cert: Vec::new(),
            address,
            dns,
            private_key: Vec::new(),
            proxy_cert: Vec::new()
        }
    }
    fn generate_ca_cert_and_key(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Generate RSA private key
        let rsa = Rsa::generate(2048)?;
        let private_key = PKey::from_rsa(rsa)?;
        let mut x509_name = openssl::x509::X509NameBuilder::new().unwrap();
        x509_name.append_entry_by_text("C", "US").unwrap();
        x509_name.append_entry_by_text("ST", "CA").unwrap();
        x509_name.append_entry_by_text("O", "Some organization").unwrap();
        x509_name.append_entry_by_text("CN", "cnm-admission-controller").unwrap();
        let x509_name = x509_name.build();
        
        let mut x509_builder = openssl::x509::X509::builder().unwrap();
        x509_builder.set_subject_name(&x509_name).unwrap();
    
        // Set the issuer name (self-signed certificate, so same as subject)
        x509_builder.set_issuer_name(&x509_name)?;
    
        let mut san_builder = SubjectAlternativeName::new();
        if let Some(dns) = &self.dns{
            san_builder.dns(dns.as_str());
        }
        san_builder.ip(self.address.as_str()); // Add IP address to SAN
        let san_extension = san_builder.build(&x509_builder.x509v3_context(None, None))?;
        x509_builder.append_extension(san_extension)?;
    
        // Set the serial number (for simplicity, using a fixed number)
        //let serial = Asn1IntegerRef{};
        //x509.set_serial_number(&serial)?;
    
        // Set the start and end dates (validity period)
        let not_before = Asn1Time::days_from_now(0)?;
        let not_after = Asn1Time::days_from_now(3650)?;
        x509_builder.set_not_before(&not_before)?;
        x509_builder.set_not_after(&not_after)?;
    
        // Set the public key and sign the certificate with the private key
        x509_builder.set_pubkey(&private_key)?;
        x509_builder.sign(&private_key, MessageDigest::sha256())?;
    
        let x509 = x509_builder.build();
    
        let pem = x509.to_pem()?;
        let key = private_key.private_key_to_pem_pkcs8()?;
    
        self.ca_cert = pem;
        self.ca_key = key;
        Ok(())
    }
    fn generate_sign_key(&mut self) -> anyhow::Result<()>{
        let rsa = Rsa::generate(2048)?;
        let private_key = PKey::from_rsa(rsa)?;
        let mut x509_builder = openssl::x509::X509::builder().unwrap();

        Ok(())
    }
}

