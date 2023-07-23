use serde::{Deserialize, Serialize};
use crate::resources::bgp_router::BgpRouter;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Configuration {
    #[serde(rename = "@")]
    attributes: Attributes,
    version: String,
    protocols: Protocols,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Attributes {
    #[serde(rename = "junos:changed-seconds")]
    changed_seconds: String,
    #[serde(rename = "junos:changed-localtime")]
    changed_localtime: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Protocols {
    bgp: Bgp,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Bgp {
    group: Vec<Group>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Group {
    name: String,
    r#type: String,
    multihop: Vec<Option<()>>,
    #[serde(rename = "local-address")]
    local_address: String,
    family: Family,
    export: Vec<String>,
    #[serde(rename = "peer-as")]
    peer_as: String,
    #[serde(rename = "local-as")]
    local_as: LocalAs,
    neighbor: Vec<Neighbor>,
    #[serde(rename = "vpn-apply-export")]
    vpn_apply_export: Vec<Option<()>>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Family {
    #[serde(rename = "inet-vpn")]
    inet_vpn: InetVpn,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct InetVpn {
    unicast: Vec<Option<()>>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct LocalAs {
    #[serde(rename = "as-number")]
    as_number: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Neighbor {
    name: String,
}

impl From<BgpRouter> for Configuration{
    fn from(value: BgpRouter) -> Self {
        
        Configuration {
            protocols: Protocols{
                bgp: Bgp { 
                    group: Vec::new(),
                }
            },
            ..Default::default()
        }
    }
}
