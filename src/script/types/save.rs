use std::{collections::HashMap, path::PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    credential::{Credential, CredentialType},
    job::JobResult,
    script::{ScriptExecutor, ScriptParameterType},
};

/// Read credential, splits by newline and save as array to parameters
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default, JsonSchema)]
pub struct SaveAsArrayFromCredential {
    pub credential_id: String,
    pub variable_name: String,
}

impl ScriptExecutor for SaveAsArrayFromCredential {
    fn execute(
        &self,
        parameters: &mut HashMap<String, ScriptParameterType>,
        _directory: PathBuf,
        _step_name: &str,
        _job_result: &mut JobResult,
    ) -> Result<(), String> {
        let credential = Credential::get(&self.credential_id).ok_or("Credential not found")?;

        let value = match &credential.value {
            CredentialType::Text(text) => text.value.clone(),
            _ => return Err("Credential type is not text".to_string()),
        };

        let array: Vec<String> = value.lines().map(|s| s.to_string()).collect();
        parameters.insert(
            format!("variables.{}", self.variable_name),
            ScriptParameterType::StringArray(array),
        );

        Ok(())
    }
}
