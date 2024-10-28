use std::{collections::HashMap, path::Path};

use crate::{job::JobResult, log::LogLevel};

use super::{models::RunningScriptStep, types::ScriptType, ScriptParameterType};

pub struct ScriptExecutionContext<'a> {
    pub parameters: &'a mut HashMap<String, ScriptParameterType>,
    pub directory: &'a Path,
    pub step_name: &'a str,
    pub job_result: &'a mut JobResult,
}

pub trait ScriptExecutor {
    fn execute(&self, context: &mut ScriptExecutionContext) -> Result<(), String>;
}

impl ScriptExecutor for RunningScriptStep {
    fn execute(&self, context: &mut ScriptExecutionContext) -> Result<(), String> {
        context
            .job_result
            .add_log(LogLevel::Info, format!("Executing step: {}", context.step_name));
        for value in self.values.iter() {
            value.execute(context)?;
        }
        Ok(())
    }
}

impl ScriptExecutor for ScriptType {
    fn execute(&self, context: &mut ScriptExecutionContext) -> Result<(), String> {
        match self {
            ScriptType::Bash(bash) => bash.execute(context),
            ScriptType::GitClone(git_clone) => git_clone.execute(context),
            ScriptType::Sync(sync) => sync.execute(context),
            ScriptType::DockerBuild(docker_build) => docker_build.execute(context),
            ScriptType::DockerStop(docker_stop) => docker_stop.execute(context),
            ScriptType::DockerRun(docker_run) => docker_run.execute(context),
        }
    }
}
