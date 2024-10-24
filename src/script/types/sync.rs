use std::{collections::HashMap, path::PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    job::JobResult,
    script::{ScriptExecutor, ScriptParameterType},
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
        let is_variable = self.directory.starts_with('$');
        let mut param_directory = if is_variable {
            let p = parameters.get(&self.directory).cloned();
            match p {
                Some(ScriptParameterType::String(s)) => PathBuf::from(s),
                _ => return Err("Could not get directory".to_string()),
            }
        } else {
            PathBuf::from(&self.directory)
        };

        if !param_directory.exists() {
            return Err(format!("Directory does not exist: {:?}", param_directory));
        }

        if param_directory.is_relative() {
            param_directory = directory.join(param_directory);
        }

        settings::sync(param_directory, job_result)
    }
}
