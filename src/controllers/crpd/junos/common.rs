use serde::{Deserialize, Serialize};
use garde::Validate;
use schemars::JsonSchema;
use super::interface;

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Unicast {
    #[serde(rename = "rib-group")]
    #[garde(skip)]
    rib_group: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct RouteAttributes {
    #[garde(skip)]
    community: Community,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Community {
    #[garde(skip)]
    #[serde(rename = "import-action")]
    import_action: String,
    #[garde(skip)]
    #[serde(rename = "export-action")]
    export_action: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct VrfTarget {
    #[garde(skip)]
    community: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Root{
    pub configuration: Configuration
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Configuration{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interfaces: Option<Interface>
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Interface{
    pub interface: Option<Vec<interface::Interface>>
}



