use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    credential::{Credential, CredentialType},
    docker::{docker_build, docker_run, docker_stop_and_rm},
    script::{
        utils::{ParameterSubstitution, SubstitutionResult},
        ScriptExecutionContext, ScriptExecutor,
    },
};
use async_trait::async_trait;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DockerBuildScript {
    pub image: String,
    pub dockerfile: Option<String>,
}

#[async_trait]
impl ScriptExecutor for DockerBuildScript {
    async fn execute(&self, context: &mut ScriptExecutionContext<'_>) -> Result<(), String> {
        // Get image name with parameter substitution
        let image = self
            .image
            .substitute_parameters(context.parameters, false)?
            .ok_or("Image name is required")?;
        let image = match image {
            SubstitutionResult::Single(s) => s,
            SubstitutionResult::Multiple(_) => {
                return Err("Image name parameter cannot be an array".to_string());
            }
        };

        // Get dockerfile path with parameter substitution
        let dockerfile = match &self.dockerfile {
            Some(dockerfile) => {
                match dockerfile
                    .substitute_parameters(context.parameters, false)?
                    .ok_or("Dockerfile path is required")?
                {
                    SubstitutionResult::Single(s) => s,
                    SubstitutionResult::Multiple(_) => {
                        return Err("Dockerfile path parameter cannot be an array".to_string());
                    }
                }
            }
            None => "Dockerfile".to_string(),
        };

        let dockerfile_path = if cfg!(windows) {
            if dockerfile.chars().nth(1) == Some(':') {
                PathBuf::from(dockerfile)
            } else {
                context.directory.join(dockerfile)
            }
        } else if dockerfile.starts_with('/') {
            PathBuf::from(dockerfile)
        } else {
            context.directory.join(dockerfile)
        };

        if !context.job_result.dry_run && !dockerfile_path.exists() {
            return Err(format!(
                "Dockerfile does not exist at path: {}",
                dockerfile_path.display()
            ));
        }
        tokio::task::yield_now().await;
        docker_build(&image, &dockerfile_path, context).await
    }
}

/// Stops and removes a docker container. Ignoring errors.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DockerStopScript {
    pub container: String,
}

#[async_trait]
impl ScriptExecutor for DockerStopScript {
    async fn execute(&self, context: &mut ScriptExecutionContext<'_>) -> Result<(), String> {
        // Get container name with parameter substitution
        let container = self
            .container
            .substitute_parameters(context.parameters, false)?
            .ok_or("Container name is required")?;
        let container = match container {
            SubstitutionResult::Single(s) => s,
            SubstitutionResult::Multiple(_) => {
                return Err("Container name parameter cannot be an array".to_string());
            }
        };

        tokio::task::yield_now().await;
        docker_stop_and_rm(&container, context).await;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum DockerRunArg {
    Direct(String),
    EnvFromCredential { credential_id: String },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DockerRunScript {
    pub image: String,
    pub container: Option<String>,
    pub args: Vec<DockerRunArg>,
}

#[async_trait]
impl ScriptExecutor for DockerRunScript {
    async fn execute(&self, context: &mut ScriptExecutionContext<'_>) -> Result<(), String> {
        // Get image name with parameter substitution
        let image = self
            .image
            .substitute_parameters(context.parameters, false)?
            .ok_or("Image name is required")?;
        let image = match image {
            SubstitutionResult::Single(s) => s,
            SubstitutionResult::Multiple(_) => {
                return Err("Image name parameter cannot be an array".to_string());
            }
        };

        let mut final_args: Vec<String> = Vec::new();

        // Add container name if specified
        if let Some(container_name) = &self.container {
            let name = container_name
                .substitute_parameters(context.parameters, false)?
                .ok_or("Container name substitution failed")?;
            let name = match name {
                SubstitutionResult::Single(s) => s,
                SubstitutionResult::Multiple(_) => {
                    return Err("Container name parameter cannot be an array".to_string());
                }
            };
            final_args.push("--name".to_string());
            final_args.push(name);
        }

        // Process each argument
        for arg in &self.args {
            match arg {
                DockerRunArg::Direct(arg_str) => {
                    let processed_arg = arg_str
                        .substitute_parameters(context.parameters, false)?
                        .ok_or("Argument substitution failed")?;
                    match processed_arg {
                        SubstitutionResult::Single(s) => final_args.push(s),
                        SubstitutionResult::Multiple(a) => {
                            for s in a {
                                final_args.push(s);
                            }
                        }
                    }
                }
                DockerRunArg::EnvFromCredential { credential_id } => {
                    let credential_id_resolved = credential_id.substitute_parameters(context.parameters, true)?;
                    if let Some(SubstitutionResult::Single(id)) = credential_id_resolved {
                        if let Some(credential) = Credential::get(&id, Some(context.job_result))? {
                            match credential.value {
                                CredentialType::Env(env) => {
                                    for line in env.value.lines() {
                                        let key = match line.split('=').next() {
                                            Some(k) => k,
                                            None => return Err("Invalid env credential: missing key".to_string()),
                                        };
                                        let value = line[key.len() + 1..].trim();
                                        final_args.push("--env".to_string());
                                        final_args.push(format!("\"{}={}\"", key, value));
                                    }
                                }
                                _ => return Err("Credential is not of type Env".to_string()),
                            }
                        }
                    }
                }
            }
        }

        // Convert to &str for docker_run function
        let args_ref: Vec<&str> = final_args.iter().map(|s| s.as_str()).collect();

        tokio::task::yield_now().await;
        docker_run(&image, args_ref, context).await
    }
}
