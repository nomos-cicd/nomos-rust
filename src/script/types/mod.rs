pub mod bash;
pub mod git;
pub mod sync;
pub mod docker;

pub use bash::BashScript;
pub use git::GitCloneScript;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use sync::SyncScript;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, JsonSchema)]
#[serde(tag = "type")]
pub enum ScriptType {
    #[serde(rename = "bash")]
    Bash(BashScript),
    #[serde(rename = "git-clone")]
    GitClone(GitCloneScript),
    #[serde(rename = "sync")]
    Sync(SyncScript),
    #[serde(rename = "docker-build")]
    DockerBuild(docker::DockerBuildScript),
}

impl ScriptType {
    #[allow(dead_code)]
    pub fn from_str(t: &str) -> Result<Self, String> {
        match t {
            "bash" => Ok(ScriptType::Bash(BashScript::default())),
            "git-clone" => Ok(ScriptType::GitClone(GitCloneScript::default())),
            "sync" => Ok(ScriptType::Sync(SyncScript::default())),
            "docker-build" => Ok(ScriptType::DockerBuild(docker::DockerBuildScript::default())),
            _ => Err(format!("Unknown script type: {}", t)),
        }
    }
}
