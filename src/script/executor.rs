use std::{collections::HashMap, path::PathBuf};

use crate::job::JobResult;

use super::{models::ScriptStep, types::ScriptType, ScriptParameterType};

pub trait ScriptExecutor {
    fn execute(
        &self,
        parameters: &mut HashMap<String, ScriptParameterType>,
        directory: PathBuf,
        step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String>;
}

impl ScriptExecutor for ScriptStep {
    fn execute(
        &self,
        parameters: &mut HashMap<String, ScriptParameterType>,
        directory: PathBuf,
        step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        for value in self.values.iter() {
            value.execute(parameters, directory.clone(), step_name, job_result)?;
        }
        Ok(())
    }
}

impl ScriptExecutor for ScriptType {
    fn execute(
        &self,
        parameters: &mut HashMap<String, ScriptParameterType>,
        directory: PathBuf,
        step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        match self {
            ScriptType::Bash(bash) => bash.execute(parameters, directory, step_name, job_result),
            ScriptType::GitClone(git_clone) => git_clone.execute(parameters, directory, step_name, job_result),
            ScriptType::Sync(sync) => sync.execute(parameters, directory, step_name, job_result),
            ScriptType::SaveAsArrayFromCredential(save_as_array_from_credential) => {
                save_as_array_from_credential.execute(parameters, directory, step_name, job_result)
            }
            ScriptType::DockerBuild(docker_build) => docker_build.execute(parameters, directory, step_name, job_result),
            ScriptType::DockerStop(docker_stop) => docker_stop.execute(parameters, directory, step_name, job_result),
            ScriptType::DockerRun(docker_run) => docker_run.execute(parameters, directory, step_name, job_result),
        }
    }
}
