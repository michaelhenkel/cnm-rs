use super::common;

use serde::{Deserialize, Serialize};
use garde::Validate;
use schemars::JsonSchema;

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Family {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    #[serde(rename = "inet-vpn")]
    inet_vpn: InetVpnFamily,
    #[garde(skip)]
    inet: InetFamily,
    #[garde(skip)]
    inet6: Inet6Family,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct InetVpnFamily {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    unicast: Option<common::Unicast>,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct InetFamily {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    unicast: Option<common::Unicast>,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Inet6Family {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    unicast: Option<common::Unicast>,
}