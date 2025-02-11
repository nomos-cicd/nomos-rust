pub mod bash;
pub mod docker;
pub mod git;
pub mod sync;

pub use bash::BashScript;
pub use git::{GitCloneScript, GitPullScript};
use serde::{Deserialize, Serialize};
pub use sync::SyncScript;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum ScriptType {
    #[serde(rename = "bash")]
    Bash(BashScript),
    #[serde(rename = "git-clone")]
    GitClone(GitCloneScript),
    #[serde(rename = "git-pull")]
    GitPull(GitPullScript),
    #[serde(rename = "sync")]
    Sync(SyncScript),
    #[serde(rename = "docker-build")]
    DockerBuild(docker::DockerBuildScript),
    #[serde(rename = "docker-stop")]
    DockerStop(docker::DockerStopScript),
    #[serde(rename = "docker-run")]
    DockerRun(docker::DockerRunScript),
}
