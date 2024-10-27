use std::{collections::HashMap, path::PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    job::JobResult,
    log::LogLevel,
    script::{
        utils::{ParameterSubstitution, SubstitutionResult},
        ScriptExecutor, ScriptParameterType,
    },
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
        directory: &PathBuf,
        _step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        // Replace all parameter references in the code
        let replaced_code = self.code.substitute_parameters(parameters, false)?;
        if replaced_code.is_none() {
            return Ok(());
        }
        let replaced_code = replaced_code.unwrap();
        let replaced_code = match replaced_code {
            SubstitutionResult::Single(s) => s,
            SubstitutionResult::Multiple(_) => {
                return Err("Code parameter cannot be an array".to_string());
            }
        };

        let original_lines = self.code.lines().collect::<Vec<&str>>();
        let lines = replaced_code.lines();
        let mut i = 0;
        for line in lines {
            if line.is_empty() {
                i += 1;
                continue;
            }
            job_result.add_log(LogLevel::Info, format!("command: {}", original_lines[i]));
            if !job_result.dry_run {
                execute_command(line, directory, job_result)?;
            }
            i += 1;
        }

        Ok(())
    }
}
