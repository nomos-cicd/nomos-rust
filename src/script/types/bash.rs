use std::{collections::HashMap, path::PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    job::JobResult,
    script::{ScriptExecutor, ScriptParameterType},
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
        let mut replaced_code = self.code.clone();
        for (key, value) in parameters.iter() {
            let value_str;
            let value = match value {
                ScriptParameterType::String(s) => s,
                ScriptParameterType::Password(p) => p,
                ScriptParameterType::Credential(c) => c,
                ScriptParameterType::Boolean(b) => {
                    value_str = b.to_string();
                    &value_str
                }
                ScriptParameterType::Number(n) => {
                    value_str = n.to_string();
                    &value_str
                }
            };
            replaced_code = replaced_code.replace(key, value);
        }

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
