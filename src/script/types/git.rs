use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    git::git_clone,
    job::JobResult,
    script::{
        utils::{ParameterSubstitution, SubstitutionResult},
        ScriptExecutor, ScriptParameterType,
    },
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct GitCloneScript {
    pub url: String,
    pub credential_id: Option<String>,
    pub branch: Option<String>,
}

impl ScriptExecutor for GitCloneScript {
    fn execute(
        &self,
        parameters: &mut HashMap<String, ScriptParameterType>,
        directory: &Path,
        step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        // Substitute parameters
        let url = self
            .url
            .substitute_parameters(parameters, false)?
            .ok_or("URL is required")?;
        let url = match url {
            SubstitutionResult::Single(s) => s,
            SubstitutionResult::Multiple(_) => {
                return Err("URL parameter cannot be an array".to_string());
            }
        };

        let credential_id = match &self.credential_id {
            Some(id) => id.substitute_parameters(parameters, true)?,
            None => None,
        };
        let credential_id = match credential_id {
            Some(id) => match id {
                SubstitutionResult::Single(s) => Some(s),
                SubstitutionResult::Multiple(_) => {
                    return Err("Credential ID parameter cannot be an array".to_string());
                }
            },
            None => None,
        };

        let branch: String = match &self.branch {
            Some(b) => {
                let branch = b.substitute_parameters(parameters, false)?;
                match branch {
                    Some(b) => match b {
                        SubstitutionResult::Single(s) => s,
                        SubstitutionResult::Multiple(_) => {
                            return Err("Branch parameter cannot be an array".to_string());
                        }
                    },
                    None => "main".to_string(),
                }
            }
            None => "main".to_string(),
        };

        git_clone(&url, branch.as_str(), directory, credential_id.as_deref(), job_result)?;

        let mut new_dir = match url.split('/').last() {
            Some(last_part) => directory.join(last_part),
            None => return Err("Invalid URL format".to_string()),
        };

        if let Some(dir_str) = new_dir.to_str() {
            if dir_str.ends_with(".git") {
                new_dir = match dir_str.strip_suffix(".git") {
                    Some(stripped) => PathBuf::from(stripped),
                    None => return Err("Failed to strip .git suffix".to_string()),
                };
            }
        } else {
            return Err("Invalid directory path".to_string());
        }

        let new_dir_str = match new_dir.to_str() {
            Some(s) => s,
            None => return Err("Invalid directory path".to_string()),
        };

        parameters.insert(
            format!("steps.{}.git-clone.directory", step_name),
            ScriptParameterType::String(new_dir_str.to_string()),
        );

        Ok(())
    }
}
