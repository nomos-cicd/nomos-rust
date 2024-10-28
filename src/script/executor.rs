use std::{collections::HashMap, path::PathBuf};

use crate::{job::JobResult, log::LogLevel};

use super::{models::RunningScriptStep, types::ScriptType, ScriptParameterType};

pub trait ScriptExecutor {
    fn execute(
        &self,
        parameters: &mut HashMap<String, ScriptParameterType>,
        directory: &PathBuf,
        step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String>;
}

impl ScriptExecutor for RunningScriptStep {
    fn execute(
        &self,
        parameters: &mut HashMap<String, ScriptParameterType>,
        directory: &PathBuf,
        step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        job_result.add_log(LogLevel::Info, format!("Executing step: {}", step_name));
        for value in self.values.iter() {
            value.execute(parameters, directory, step_name, job_result)?;
        }
        Ok(())
    }
}

impl ScriptExecutor for ScriptType {
    fn execute(
        &self,
        parameters: &mut HashMap<String, ScriptParameterType>,
        directory: &PathBuf,
        step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        match self {
            ScriptType::Bash(bash) => bash.execute(parameters, directory, step_name, job_result),
            ScriptType::GitClone(git_clone) => git_clone.execute(parameters, directory, step_name, job_result),
            ScriptType::Sync(sync) => sync.execute(parameters, directory, step_name, job_result),
            ScriptType::DockerBuild(docker_build) => docker_build.execute(parameters, directory, step_name, job_result),
            ScriptType::DockerStop(docker_stop) => docker_stop.execute(parameters, directory, step_name, job_result),
            ScriptType::DockerRun(docker_run) => docker_run.execute(parameters, directory, step_name, job_result),
        }
    }
}
