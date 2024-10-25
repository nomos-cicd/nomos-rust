pub mod bash;
pub mod docker;
pub mod git;
pub mod sync;

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
    #[serde(rename = "docker-stop")]
    DockerStop(docker::DockerStopScript),
    #[serde(rename = "docker-run")]
    DockerRun(docker::DockerRunScript),
}

impl ScriptType {
    #[allow(dead_code)]
    pub fn from_str(t: &str) -> Result<Self, String> {
        match t {
            "bash" => Ok(ScriptType::Bash(BashScript::default())),
            "git-clone" => Ok(ScriptType::GitClone(GitCloneScript::default())),
            "sync" => Ok(ScriptType::Sync(SyncScript::default())),
            "docker-build" => Ok(ScriptType::DockerBuild(docker::DockerBuildScript::default())),
            "docker-stop" => Ok(ScriptType::DockerStop(docker::DockerStopScript::default())),
            "docker-run" => Ok(ScriptType::DockerRun(docker::DockerRunScript::default())),
            _ => Err(format!("Unknown script type: {}", t)),
        }
    }
}
