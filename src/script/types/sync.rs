use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    script::{
        utils::{ParameterSubstitution, SubstitutionResult},
        ScriptExecutionContext, ScriptExecutor,
    },
    settings,
};
use async_trait::async_trait;

/// Scans directory for credential, script and job files and syncs them with the database.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SyncScript {
    pub directory: String,
}

#[async_trait]
impl ScriptExecutor for SyncScript {
    async fn execute(&self, context: &mut ScriptExecutionContext<'_>) -> Result<(), String> {
        // Get directory with parameter substitution
        let param_directory_str = self
            .directory
            .substitute_parameters(context.parameters, false)?
            .ok_or("Directory is required")?;
        let param_directory_str = match param_directory_str {
            SubstitutionResult::Single(s) => s,
            SubstitutionResult::Multiple(_) => {
                return Err("Directory parameter cannot be an array".to_string());
            }
        };

        let mut param_directory = PathBuf::from(param_directory_str);

        if !context.job_result.dry_run && !param_directory.exists() {
            return Err(format!("Directory does not exist: {:?}", param_directory));
        }

        if param_directory.is_relative() {
            param_directory = context.directory.join(param_directory);
        }

        tokio::task::yield_now().await;
        settings::sync(param_directory, context.job_result).await
    }
}
