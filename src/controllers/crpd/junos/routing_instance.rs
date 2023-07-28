use garde::Validate;
use schemars::JsonSchema;
use super::interface;
use super::routing_options;
use super::protocol;
use super::common;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct RoutingInstances {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    instance: Vec<Instance>,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Instance {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    name: String,
    #[garde(skip)]
    instance_type: String,
    #[garde(skip)]
    routing_options: Option<routing_options::RoutingOptions>,
    #[garde(skip)]
    protocols: Option<protocol::Protocols>,
    #[garde(skip)]
    interface: Option<Vec<interface::Interface>>,
    #[garde(skip)]
    vrf_import: Option<Vec<String>>,
    #[garde(skip)]
    vrf_target: Option<Vec<common::VrfTarget>>,
    #[garde(skip)]
    vrf_table_label: Option<Vec<String>>,
}
