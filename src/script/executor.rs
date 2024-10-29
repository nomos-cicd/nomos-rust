use std::{collections::HashMap, path::Path};

use tokio_util::sync::CancellationToken;

use crate::{job::JobResult, log::LogLevel};

use super::{models::RunningScriptStep, types::ScriptType, ScriptParameterType};

pub struct ScriptExecutionContext<'a> {
    pub parameters: &'a mut HashMap<String, ScriptParameterType>,
    pub directory: &'a Path,
    pub step_name: &'a str,
    pub job_result: &'a mut JobResult,
    pub cancellation_token: &'a CancellationToken,
}

pub trait ScriptExecutor {
    async fn execute(&self, context: &mut ScriptExecutionContext<'_>) -> Result<(), String>;
}

impl ScriptExecutor for RunningScriptStep {
    async fn execute(&self, context: &mut ScriptExecutionContext<'_>) -> Result<(), String> {
        context
            .job_result
            .add_log(LogLevel::Info, format!("Executing step: {}", context.step_name));
        for value in self.values.iter() {
            value.execute(context).await?;
        }
        Ok(())
    }
}

impl ScriptExecutor for ScriptType {
    async fn execute(&self, context: &mut ScriptExecutionContext<'_>) -> Result<(), String> {
        match self {
            ScriptType::Bash(bash) => bash.execute(context).await,
            ScriptType::GitClone(git_clone) => git_clone.execute(context).await,
            ScriptType::Sync(sync) => sync.execute(context).await,
            ScriptType::DockerBuild(docker_build) => docker_build.execute(context).await,
            ScriptType::DockerStop(docker_stop) => docker_stop.execute(context).await,
            ScriptType::DockerRun(docker_run) => docker_run.execute(context).await,
        }
    }
}
