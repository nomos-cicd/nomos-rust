use std::{collections::HashMap, path::PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    git::git_clone,
    job::JobResult,
    log::LogLevel,
    script::{ScriptExecutor, ScriptParameterType},
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
        let mut url = self.url.clone();
        let is_variable = url.starts_with("$parameters.");
        if is_variable {
            let p = parameters.get(&url).cloned();
            match p {
                Some(ScriptParameterType::String(s)) => url = s,
                _ => return Err("Could not get url".to_string()),
            }
        }

        let mut credential_id = self.credential_id.clone();
        let is_variable = credential_id
            .as_ref()
            .map_or(false, |id| id.starts_with("$parameters."));
        if is_variable {
            let p = parameters.get(credential_id.as_ref().unwrap()).cloned();
            match p {
                Some(ScriptParameterType::Credential(s)) => credential_id = Some(s),
                _ => {
                    credential_id = None;
                }
            }
        }

        let mut branch = self.branch.clone();
        let is_variable = branch.as_ref().map_or(false, |b| b.starts_with("$parameters."));
        if is_variable {
            let p = parameters.get(branch.as_ref().unwrap()).cloned();
            match p {
                Some(ScriptParameterType::String(s)) => branch = Some(s),
                _ => {
                    branch = None;
                    job_result.add_log(
                        LogLevel::Warning,
                        format!("Could not get branch, using default: {:?}", branch),
                    );
                }
            }
        } else if branch.is_none() {
            branch = "main".to_string().into();
        }

        git_clone(
            &url,
            branch.unwrap().as_str(),
            directory.clone(),
            credential_id.as_deref(),
            job_result,
        )
        .map_err(|e| e.to_string())?;

        let mut cloned_dir = directory.clone().join(url.split('/').last().unwrap());
        if cloned_dir.to_str().unwrap().ends_with(".git") {
            cloned_dir = cloned_dir.parent().unwrap().to_path_buf();
        }

        parameters.insert(
            format!("$steps.{}.git-clone.directory", step_name),
            ScriptParameterType::String(cloned_dir.to_str().unwrap().to_string()),
        );

        Ok(())
    }
}
