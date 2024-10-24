use std::{collections::HashMap, path::PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    job::JobResult,
    script::{utils::ParameterSubstitution, ScriptExecutor, ScriptParameterType},
    utils::execute_command,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, JsonSchema)]
pub struct BashScript {
    pub code: String,
}

impl ScriptExecutor for BashScript {
    fn execute(
        &self,
        parameters: &mut HashMap<String, ScriptParameterType>,
        directory: PathBuf,
        _step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        // Replace all parameter references in the code
        let replaced_code = self.code.substitute_parameters(parameters, false)?.unwrap();

        // Split into lines and execute each command
        let binding = replaced_code.replace("\r\n", "\n");
        let lines = binding.split('\n');
        for line in lines {
            if line.is_empty() {
                continue;
            }
            execute_command(line, directory.clone(), job_result)
                .map_err(|e| format!("Error executing command: {}", e))?;
        }

        Ok(())
    }
}
