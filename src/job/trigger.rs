use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, JsonSchema, Default)]
pub struct ManualTriggerParameter {}

#[derive(Debug, Deserialize, Clone)]
pub struct GithubRepository {
    pub full_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GithubPayload {
    pub repository: GithubRepository,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, JsonSchema)]
pub struct GithubTriggerParameter {
    pub branch: String,
    pub events: Vec<String>,
    pub secret_credential_id: String,
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

impl Default for GithubTriggerParameter {
    fn default() -> Self {
        GithubTriggerParameter {
            branch: "main".to_string(),
            events: vec!["push".to_string()],
            secret_credential_id: "".to_string(),
            url: "git@github.com:godotengine/godot.git".to_string(),
        }
    }
}
