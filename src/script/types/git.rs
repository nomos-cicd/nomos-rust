use std::{collections::HashMap, path::PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    git::git_clone,
    job::JobResult,
    script::{utils::ParameterSubstitution, ScriptExecutor, ScriptParameterType},
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default, JsonSchema)]
pub struct GitCloneScript {
    pub url: String,
    pub credential_id: Option<String>,
    pub branch: Option<String>,
}

impl ScriptExecutor for GitCloneScript {
    fn execute(
        &self,
        parameters: &mut HashMap<String, ScriptParameterType>,
        directory: PathBuf,
        step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        // Substitute parameters
        let url = self.url.substitute_parameters(parameters, false)?.ok_or("URL is required")?;

        let credential_id = match &self.credential_id {
            Some(id) => id.substitute_parameters(parameters, true)?,
            None => None,
        };

        let branch = match &self.branch {
            Some(b) => {
                let res = b.substitute_parameters(parameters, false)?;
                res.unwrap_or_else(|| "main".to_string())
            }
            None => "main".to_string(),
        };

        git_clone(
            &url,
            branch.as_str(),
            directory.clone(),
            credential_id.as_deref(),
            job_result,
        )
        .map_err(|e| e.to_string())?;

        let mut cloned_dir = directory.clone().join(url.split('/').last().unwrap());
        if cloned_dir.to_str().unwrap().ends_with(".git") {
            cloned_dir = PathBuf::from(cloned_dir.to_str().unwrap().strip_suffix(".git").unwrap());
        }

        parameters.insert(
            format!("steps.{}.git-clone.directory", step_name),
            ScriptParameterType::String(cloned_dir.to_str().unwrap().to_string()),
        );

        Ok(())
    }
}
