use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, JsonSchema, Default)]
pub struct ManualTriggerParameter {}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, JsonSchema)]
pub struct GithubTriggerParameter {
    pub branch: String,
    pub events: Vec<String>,
    pub secret: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, JsonSchema)]
#[serde(tag = "type")]
pub enum TriggerType {
    #[serde(rename = "manual")]
    Manual(ManualTriggerParameter),
    #[serde(rename = "github")]
    Github(GithubTriggerParameter),
}

impl TriggerType {
    pub fn get_json_schema() -> serde_json::Value {
        let schema = schema_for!(TriggerType);
        serde_json::to_value(schema).unwrap()
    }
}
