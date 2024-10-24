use std::{collections::HashMap, path::PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    docker::docker_build,
    job::JobResult,
    script::{ScriptExecutor, ScriptParameterType},
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default, JsonSchema)]
pub struct DockerBuildScript {
    pub image: String,
    pub dockerfile: String,
}

impl ScriptExecutor for DockerBuildScript {
    fn execute(
        &self,
        parameters: &mut HashMap<String, ScriptParameterType>,
        directory: PathBuf,
        _step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        let mut image = self.image.clone();
        let is_variable = image.starts_with("$parameters.");
        if is_variable {
            let p = parameters.get(&image).cloned();
            match p {
                Some(ScriptParameterType::String(s)) => image = s,
                _ => return Err("Could not get image name".to_string()),
            }
        }

        let mut dockerfile = self.dockerfile.clone();
        let is_variable = dockerfile.starts_with("$parameters.");
        if is_variable {
            let p = parameters.get(&dockerfile).cloned();
            match p {
                Some(ScriptParameterType::String(s)) => dockerfile = s,
                _ => return Err("Could not get dockerfile path".to_string()),
            }
        }

        let dockerfile_path = if dockerfile.starts_with('/') {
            PathBuf::from(dockerfile)
        } else {
            directory.join(dockerfile)
        };

        if !dockerfile_path.exists() {
            return Err(format!(
                "Dockerfile does not exist at path: {}",
                dockerfile_path.display()
            ));
        }

        docker_build(&image, dockerfile_path, directory, job_result)
    }
}
