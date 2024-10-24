use std::{collections::HashMap, path::PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    job::JobResult,
    script::{utils::ParameterSubstitution, ScriptExecutor, ScriptParameterType},
    settings,
};

/// Scans directory for credential, script and job files and syncs them with the database.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default, JsonSchema)]
pub struct SyncScript {
    pub directory: String,
}

impl ScriptExecutor for SyncScript {
    fn execute(
        &self,
        parameters: &mut HashMap<String, ScriptParameterType>,
        directory: PathBuf,
        _step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        // Get directory with parameter substitution
        let param_directory_str = self
            .directory
            .substitute_parameters(parameters, false)?
            .ok_or("Directory is required")?;

        let mut param_directory = PathBuf::from(param_directory_str);

        if !param_directory.exists() {
            return Err(format!("Directory does not exist: {:?}", param_directory));
        }

        if param_directory.is_relative() {
            param_directory = directory.join(param_directory);
        }

        settings::sync(param_directory, job_result)
    }
}
