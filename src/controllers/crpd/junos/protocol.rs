use serde::{Deserialize, Serialize};
use garde::Validate;
use schemars::JsonSchema;

use super::bgp;
use super::evpn;

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Protocols {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    bgp: bgp::Bgp,
    #[garde(skip)]
    evpn: evpn::Evpn,
}