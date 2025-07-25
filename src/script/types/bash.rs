use serde::{Deserialize, Serialize};

use crate::{
    log::LogLevel,
    script::{
        utils::{ParameterSubstitution, SubstitutionResult},
        ScriptExecutionContext, ScriptExecutor,
    },
    utils::execute_command,
};
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BashScript {
    pub code: String,
}

#[async_trait]
impl ScriptExecutor for BashScript {
    async fn execute(&self, context: &mut ScriptExecutionContext<'_>) -> Result<(), String> {
        // Replace all parameter references in the code
        let replaced_code = self.code.substitute_parameters(context.parameters, false)?;
        let replaced_code = match replaced_code {
            Some(code) => match code {
                SubstitutionResult::Single(s) => s,
                SubstitutionResult::Multiple(_) => {
                    return Err("Code parameter cannot be an array".to_string());
                }
            },
            None => return Ok(()),
        };

        let original_lines = self.code.lines().collect::<Vec<&str>>();
        let lines = replaced_code.lines();
        let mut i = 0;
        for line in lines {
            if line.is_empty() {
                i += 1;
                continue;
            }
            tokio::task::yield_now().await;
            context
                .job_result
                .add_log(LogLevel::Info, format!("command: {}", original_lines[i]));
            if !context.job_result.dry_run {
                execute_command(line, context).await?;
            }
            i += 1;
        }

        Ok(())
    }
}
