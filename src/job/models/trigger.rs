use serde::{Deserialize, Serialize};

pub trait TriggerPlaceHolder {
    fn get_place_holder() -> Self;
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ManualTriggerParameter {}

#[derive(Debug, Deserialize, Clone)]
pub struct GithubRepository {
    pub full_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GithubPayload {
    pub repository: GithubRepository,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct GithubTriggerParameter {
    pub branch: String,
    pub events: Vec<String>,
    pub secret_credential_id: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum TriggerType {
    #[serde(rename = "manual")]
    Manual(ManualTriggerParameter),
    #[serde(rename = "github")]
    Github(GithubTriggerParameter),
}

impl TriggerPlaceHolder for ManualTriggerParameter {
    fn get_place_holder() -> Self {
        ManualTriggerParameter {}
    }
}

impl TriggerPlaceHolder for GithubTriggerParameter {
    fn get_place_holder() -> Self {
        GithubTriggerParameter {
            branch: "main".to_string(),
            events: vec!["push".to_string()],
            secret_credential_id: "".to_string(),
            url: "git@github.com:godotengine/godot.git".to_string(),
        }
    }
}
