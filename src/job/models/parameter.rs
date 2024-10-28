use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::script::ScriptParameterType;

#[derive(Deserialize, Serialize, PartialEq, Clone, JsonSchema, Debug)]
pub struct JobParameterDefinition {
    pub name: String,
    pub default: Option<ScriptParameterType>,
}
