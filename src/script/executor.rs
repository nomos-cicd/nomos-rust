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
        }
    }
}
