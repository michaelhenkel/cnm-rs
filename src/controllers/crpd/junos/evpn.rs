use serde::{Deserialize, Serialize};
use garde::Validate;
use schemars::JsonSchema;
use super::common;

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Evpn {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    ip_prefix_routes: IPrefixRoutes,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct IPrefixRoutes {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    advertise: String,
    #[garde(skip)]
    encapsulation: String,
    #[garde(skip)]
    vni: u32,
    #[garde(skip)]
    export: Vec<String>,
    #[garde(skip)]
    route_attributes: common::RouteAttributes,
}