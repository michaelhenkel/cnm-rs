use serde::{Deserialize, Serialize};
use garde::Validate;
use schemars::JsonSchema;
use super::family;

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct RoutingOptions {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    #[serde(rename = "interface-routes")]
    interface_routes: InterfaceRoutes,
    #[garde(skip)]
    #[serde(rename = "auto-export")]
    auto_export: AutoExport,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct InterfaceRoutes {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    #[serde(rename = "rib-group")]
    rib_group: RIBGroup,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct RIBGroup {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    inet: String,
    #[garde(skip)]
    inet6: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
struct AutoExport {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    family: family::Family,
}