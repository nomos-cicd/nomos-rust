use std::{collections::HashMap, path::PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    docker::docker_build,
    job::JobResult,
    script::{utils::ParameterSubstitution, ScriptExecutor, ScriptParameterType},
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default, JsonSchema)]
pub struct DockerBuildScript {
    pub image: String,
    pub dockerfile: Option<String>,
}

impl ScriptExecutor for DockerBuildScript {
    fn execute(
        &self,
        parameters: &mut HashMap<String, ScriptParameterType>,
        directory: PathBuf,
        _step_name: &str,
        job_result: &mut JobResult,
    ) -> Result<(), String> {
        // Get image name with parameter substitution
        let image = self
            .image
            .substitute_parameters(parameters, false)?
            .ok_or("Image name is required")?;

        // Get dockerfile path with parameter substitution
        let dockerfile = match &self.dockerfile {
            Some(dockerfile) => dockerfile
                .substitute_parameters(parameters, true)?
                .unwrap_or_else(|| "Dockerfile".to_string()),
            None => "Dockerfile".to_string(),
        };

        let dockerfile_path = if cfg!(windows) {
            if dockerfile.chars().nth(1) == Some(':') {
                PathBuf::from(dockerfile)
            } else {
                directory.join(dockerfile)
            }
        } else {
            if dockerfile.starts_with('/') {
                PathBuf::from(dockerfile)
            } else {
                directory.join(dockerfile)
            }
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
