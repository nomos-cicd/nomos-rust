use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
    #[serde(rename = "string-array")]
    StringArray(Vec<String>),
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct ScriptParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<ScriptParameterType>,
}
