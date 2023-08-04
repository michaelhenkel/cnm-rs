
use crate::resources::{
    bgp_router,
    bgp_router_group,
    interface_group,
    interface,
    ip_address,
    vrrp_group,
    vrrp
};
use crate::controllers::controllers;
use kube::{core::{
    admission::{AdmissionRequest, AdmissionResponse, AdmissionReview},
    DynamicObject, Resource, ResourceExt,
}, Client};
use crate::cert::cert;
use k8s_openapi::api::admissionregistration::v1 as adm_v1;
use std::{convert::Infallible, error::Error, collections::HashMap};
use tracing::*;
use warp::{reply, Filter, Reply};

use k8s_openapi::ByteString;

pub struct AdmissionController{
    address: String,
    dns: String,
    client: Client,
}

impl AdmissionController{
    pub fn new(address: String, dns: String, client: Client) -> Self{
        Self{
            address,
            dns,
            client,
        }
    }
    pub async fn admission(&self) -> anyhow::Result<()>{
        info!("Starting admission controller");
        let (ca, kp) = match cert::create_ca_key_cert(self.dns.clone()){
            Ok(ca_cert_pem) => {
                ca_cert_pem
            },
            Err(e) => {
                error!("Failed to create ca key and cert: {}", e);
                return Err(e);
            }
        };
        let ca_cert = match cert::ca_string_to_certificate(ca.clone(), kp.clone(), false){
            Ok(ca_cert) => {
                ca_cert
            },
            Err(e) => {
                error!("Failed to create ca cert: {}", e);
                return Err(e);
            }
        };
        let (key, cert) = match cert::create_sign_private_key(self.dns.clone(), self.address.clone(), ca_cert){
            Ok((key, cert)) => {
                (key, cert)
            },
            Err(e) => {
                error!("Failed to create sign private key: {}", e);
                return Err(e);
            }
        };
        self.adm_registration(ca).await?;

        let routes = warp::path("mutate")
            .and(warp::body::json())
            .and_then(mutate_handler)
            .with(warp::trace::request());
    
        // You must generate a certificate for the service / url,
        // encode the CA in the MutatingWebhookConfiguration, and terminate TLS here.
        // See admission_setup.sh + admission_controller.yaml.tpl for how to do this.
        let addr = format!("{}:8443", self.address.clone());
        warp::serve(warp::post().and(routes))
            .tls()
            .cert(cert.as_bytes())
            .key(key.as_bytes())
            .run(addr.parse::<std::net::SocketAddr>()?) // local-dev
            .await;
        info!("Admission controller stopped");
        Ok(())
    }
    async fn adm_registration(&self, ca_pem: String) -> anyhow::Result<()>{
        info!("Registering admission controller mutating webhook");
        //let ca_pem_64 = general_purpose::STANDARD.encode(&ca_pem.as_bytes());
        let mut mutating_webhook_config = adm_v1::MutatingWebhookConfiguration::default();
        mutating_webhook_config.metadata.name = Some("cnm-mutating-webhook-config".to_string());
        mutating_webhook_config.webhooks = Some(vec![adm_v1::MutatingWebhook{
            name: "cnm-admission.default.svc".to_string(),
            client_config: adm_v1::WebhookClientConfig{
                url: Some(format!("https://{}:8443/mutate", self.address.clone())),
                ca_bundle: Some(ByteString(ca_pem.as_bytes().to_vec())),
                ..Default::default()
            },
            rules: Some(vec![adm_v1::RuleWithOperations{
                operations: Some(vec!["CREATE".to_string(), "UPDATE".to_string(), "DELETE".to_string()]),
                api_groups: Some(vec!["cnm.juniper.net".to_string()]),
                api_versions: Some(vec!["v1".to_string()]),
                resources: Some(vec![
                    "bgprouters".to_string(),
                    "bgproutergroups".to_string(),
                    "interfacegroups".to_string(),
                    "interfaces".to_string(),
                    "ipaddresses".to_string(),
                    "vrrpgroups".to_string(),
                ]),
                scope: Some("Namespaced".to_string()),
                ..Default::default()
            }]),
            failure_policy: Some("Fail".to_string()),
            admission_review_versions: vec!["v1".to_string()],
            side_effects: "None".to_string(),
            timeout_seconds: Some(5),
            ..Default::default()
        }]);
        controllers::create_or_update_cluster::<adm_v1::MutatingWebhookConfiguration>(mutating_webhook_config, self.client.clone()).await?;
        info!("Admission controller mutating webhook registered");
        Ok(())
    }


}

fn mutate(res: AdmissionResponse, obj: &DynamicObject) -> Result<AdmissionResponse, Box<dyn Error>> {
    if let Some(types) = &obj.types{
        info!("Kind: {}", types.kind);
        let mut labels = HashMap::new();
        match types.kind.as_str(){
            "IpAddress" => {
                info!("IpAddress: {}", obj.data);
                if let Some(spec) = obj.data.get("spec"){
                    let ip_address_spec = serde_json::from_value::<ip_address::IpAddressSpec>(spec.clone())?;
                    labels.insert("cnm.juniper.net~1pool", ip_address_spec.pool.name.as_ref().unwrap().clone());
                }
            },
            "BgpRouter" => {
                info!("BgpRouter: {}", obj.data);
                if let Some(spec) = obj.data.get("spec"){
                    let bgp_router_spec = serde_json::from_value::<bgp_router::BgpRouterSpec>(spec.clone())?;
                    match bgp_router_spec.instance_parent{
                        Some(instance_parent) => {
                            labels.insert("cnm.juniper.net~1instanceType", instance_parent.parent_type.to_string());
                        },
                        None => {}
                    }
            
                    labels.insert("cnm.juniper.net~1bgpRouterManaged", bgp_router_spec.managed.to_string());
                }
            },
            "BgpRouterGroup" => {
                info!("BgpRouterGroup: {}", obj.data);
                if let Some(spec) = obj.data.get("spec"){
                    let bgp_router_group_spec = serde_json::from_value::<bgp_router_group::BgpRouterGroupSpec>(spec.clone())?;
                    match bgp_router_group_spec.bgp_router_template.instance_parent{
                        Some(instance_parent) => {
                            labels.insert("cnm.juniper.net~1instanceSelector", instance_parent.reference.name.as_ref().unwrap().clone());
                            labels.insert("cnm.juniper.net~1instanceType", instance_parent.parent_type.to_string());
                        },
                        None => {}
                    }

                    match bgp_router_group_spec.bgp_router_template.routing_instance_parent{
                        Some(instance_parent) => {
                            labels.insert("cnm.juniper.net~1routingInstance", instance_parent.name.as_ref().unwrap().clone());
                        },
                        None => {}
                    }
                }
            },
            "Interface" => {
                info!("Interface: {}", obj.data);
                if let Some(spec) = obj.data.get("spec"){
                    let interface_spec = serde_json::from_value::<interface::InterfaceSpec>(spec.clone())?;
                    labels.insert("cnm.juniper.net~1instanceType", interface_spec.instance_parent.parent_type.to_string());
                }
            },
            "InterfaceGroup" => {
                info!("InterfaceGroup: {}", obj.data);
                if let Some(spec) = obj.data.get("spec"){
                    let interface_group_spec = serde_json::from_value::<interface_group::InterfaceGroupSpec>(spec.clone())?;
                    labels.insert("cnm.juniper.net~1instanceType", interface_group_spec.interface_template.instance_parent.parent_type.to_string());
                    labels.insert("cnm.juniper.net~1instanceSelector", interface_group_spec.interface_template.instance_parent.reference.name.as_ref().unwrap().clone());
                }
            },
            "VrrpGroup" => {
                info!("VrrpGroup: {}", obj.data);
                if let Some(spec) = obj.data.get("spec"){
                    let vrrp_group_spec = serde_json::from_value::<vrrp_group::VrrpGroupSpec>(spec.clone())?;
                    match vrrp_group_spec.vrrp_template.interface_selector{
                        vrrp::InterfaceSelector::InterfaceGroupParent(interface_group_parent) => {
                            labels.insert("cnm.juniper.net~1interfaceGroup", interface_group_parent.name.as_ref().unwrap().clone());
                        },
                        _ => {}
                    }
                }
            },
            _ => {
                info!{"Unknown type: {}", types.kind};
            }
        }

        let mut patches = Vec::new();
        info!("Labels: {:?}", labels);
        // Ensure labels exist before adding a key to it
        if obj.meta().labels.is_none() {
            patches.push(json_patch::PatchOperation::Add(json_patch::AddOperation {
                path: "/metadata/labels".into(),
                value: serde_json::json!({}),
            }));
        }
        for (p, v) in &labels {
            patches.push(json_patch::PatchOperation::Add(json_patch::AddOperation {
                path: format!("/metadata/labels/{}", p).into(),
                value: serde_json::Value::String(v.to_string()),
            }));
        }
        return Ok(res.with_patch(json_patch::Patch(patches))?);
    }
    Ok(res)
}

// A general /mutate handler, handling errors from the underlying business logic
async fn mutate_handler(body: AdmissionReview<DynamicObject>) -> Result<impl Reply, Infallible> {
    // Parse incoming webhook AdmissionRequest first
    let req: AdmissionRequest<_> = match body.try_into() {
        Ok(req) => req,
        Err(err) => {
            error!("invalid request: {}", err.to_string());
            return Ok(reply::json(
                &AdmissionResponse::invalid(err.to_string()).into_review(),
            ));
        }
    };

    // Then construct a AdmissionResponse
    let mut res = AdmissionResponse::from(&req);
    // req.Object always exists for us, but could be None if extending to DELETE events
    if let Some(obj) = req.object {
        let name = obj.name_any(); // apiserver may not have generated a name yet
        res = match mutate(res.clone(), &obj) {
            Ok(res) => {
                info!("accepted: {:?} on Foo {}", req.operation, name);
                res
            }
            Err(err) => {
                warn!("denied: {:?} on {} ({})", req.operation, name, err);
                res.deny(err.to_string())
            }
        };
    };
    // Wrap the AdmissionResponse wrapped in an AdmissionReview
    Ok(reply::json(&res.into_review()))
}


// The main handler and core business logic, failures here implies rejected applies


