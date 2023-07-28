use garde::Validate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use super::family;
use super::protocol;

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Configuration {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    pub protocols: protocol::Protocols,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Attributes {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    #[serde(rename = "junos:changed-seconds")]
    changed_seconds: String,
    #[serde(rename = "junos:changed-localtime")]
    #[garde(skip)]
    changed_localtime: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Bgp {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    group: Vec<Group>,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Group {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    name: String,
    #[garde(skip)]
    r#type: String,
    #[garde(skip)]
    multihop: Vec<Option<()>>,
    #[serde(rename = "local-address")]
    #[garde(skip)]
    local_address: String,
    #[garde(skip)]
    family: family::Family,
    #[garde(skip)]
    export: Vec<String>,
    #[serde(rename = "peer-as")]
    #[garde(skip)]
    peer_as: String,
    #[serde(rename = "local-as")]
    #[garde(skip)]
    local_as: LocalAs,
    #[garde(skip)]
    neighbor: Vec<Neighbor>,
    #[garde(skip)]
    multipath: Vec<Option<()>>,
    #[garde(skip)]
    #[serde(rename = "vpn-apply-export")]
    vpn_apply_export: Vec<Option<()>>,
}


#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct LocalAs {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    #[serde(rename = "as-number")]
    as_number: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Neighbor {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    name: String,
}

/*
impl From<BgpRouter> for Configuration{
    fn from(bgp_router: BgpRouter) -> Self {
        let mut peer_map = HashMap::new();

        let spec_peers = if let Some(bgp_references) = bgp_router.spec.bgp_peer_references{
            bgp_references
        } else {
            Vec::new()
        };

        let status_peers = if bgp_router.status.is_some(){
            if let Some(bgp_references) = bgp_router.status.unwrap().bgp_peer_references{
                bgp_references
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let peer_list = combine_vec(spec_peers, status_peers);


        peer_list.iter().for_each(|peer| {
            let mut neighbor = Neighbor::default();
            neighbor.name = peer.peer_reference.name.as_ref().unwrap().clone();
            let group_name = if let Some(group) = &peer.bgp_router_group{
                group.clone()
            } else {
                "default".to_string()
            };
            peer_map.insert(group_name, neighbor);
        });
        
        
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
*/

// combine vec combines two vecs into one
fn combine_vec<T>(mut vec1: Vec<T>, mut vec2: Vec<T>) -> Vec<T>{
     vec1.append(&mut vec2);
     vec1
}
