use serde::{Deserialize, Serialize};
use garde::Validate;
use schemars::JsonSchema;

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Interface {
    #[schemars(length(min = 1))]
    #[garde(skip)]
    name: String,
}