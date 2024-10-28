use serde::{Deserialize, Serialize};

use crate::script::ScriptParameterType;

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub struct JobParameterDefinition {
    pub name: String,
    pub default: Option<ScriptParameterType>,
}
