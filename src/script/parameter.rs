use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(tag = "type", content = "value")]
pub enum ScriptParameterType {
    #[serde(rename = "string")]
    String(String),
    #[serde(rename = "boolean")]
    Boolean(bool),
    #[serde(rename = "number")]
    Number(i64),
    #[serde(rename = "password")]
    Password(String),
    #[serde(rename = "credential")]
    Credential(String),
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Default, JsonSchema, Debug)]
pub struct ScriptParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<ScriptParameterType>,
}

impl ScriptParameterType {
    pub fn get_json_schema() -> serde_json::Value {
        let schema = schema_for!(ScriptParameterType);
        serde_json::to_value(schema).unwrap()
    }
}
