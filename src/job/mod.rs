mod job_result;
mod trigger;
mod utils;

pub use job_result::JobResult;
pub use trigger::{GithubPayload, TriggerType};
pub use utils::{default_job_results_location, default_jobs_location, next_job_result_id};

use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
};

use crate::{
    log::LogLevel,
    script::{models::Script, ScriptExecutor, ScriptParameter, ScriptParameterType},
};

#[derive(Deserialize, Serialize, PartialEq, Clone, JsonSchema, Debug)]
pub struct JobParameterDefinition {
    pub name: String,
    pub default: Option<ScriptParameterType>,
}

#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub parameters: Vec<JobParameterDefinition>,
    pub triggers: Vec<TriggerType>,
    pub script_id: String,
    pub read_only: bool,
}

impl Job {
    fn get_script(&self, script: Option<&Script>) -> Result<Script, String> {
        match script {
            Some(script) => Ok(script.clone()),
            None => Script::get(&self.script_id)?.ok_or_else(|| format!("Script not found: {}", self.script_id)),
        }
    }

    pub fn get(id: &str) -> Result<Option<Self>, String> {
        let path = default_jobs_location()?.join(format!("{}.yml", id));
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read job file: {}", e))?;
        serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse job YAML: {}", e))
    }

    pub fn get_all() -> Result<Vec<Self>, String> {
        let path = default_jobs_location()?;
        let mut jobs = Vec::new();

        for entry in fs::read_dir(path).map_err(|e| format!("Failed to read jobs directory: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            match Job::try_from(path.clone()) {
                Ok(job) => jobs.push(job),
                Err(e) => eprintln!("Error reading job from {:?}: {}", path, e),
            }
        }

        Ok(jobs)
    }

    pub fn sync(&self, job_result: Option<&mut JobResult>) -> Result<(), String> {
        self.validate(None, Default::default())?;

        match Job::get(&self.id)? {
            Some(existing_job) => {
                let needs_update = existing_job.name != self.name
                    || existing_job.parameters != self.parameters
                    || existing_job.triggers != self.triggers
                    || existing_job.script_id != self.script_id;

                if needs_update {
                    self.save()?;
                    if let Some(result) = job_result {
                        result.add_log(LogLevel::Info, format!("Updated job {}", self.id));
                    }
                } else if let Some(result) = job_result {
                    result.add_log(LogLevel::Info, format!("No changes in job {}", self.id));
                }
            }
            None => {
                self.save()?;
                if let Some(result) = job_result {
                    result.add_log(LogLevel::Info, format!("Created job {}", self.id));
                }
            }
        }

        Ok(())
    }

    fn save(&self) -> Result<(), String> {
        let path = default_jobs_location()?.join(format!("{}.yml", self.id));
        let file = File::create(&path).map_err(|e| format!("Failed to create job file {}: {}", path.display(), e))?;

        serde_yaml::to_writer(file, self).map_err(|e| format!("Failed to write job YAML: {}", e))
    }

    pub fn delete(&self) -> Result<(), String> {
        let path = PathBuf::from("jobs").join(format!("{}.yml", self.id));
        fs::remove_file(&path).map_err(|e| format!("Failed to delete job file {}: {}", path.display(), e))
    }

    pub fn execute(&self, parameters: HashMap<String, ScriptParameterType>) -> Result<String, String> {
        let script = self.get_script(None)?;
        self.execute_with_script(parameters, &script)
    }

    pub fn execute_with_script(
        &self,
        parameters: HashMap<String, ScriptParameterType>,
        script: &Script,
    ) -> Result<String, String> {
        self.validate(Some(script), parameters.clone())?;

        let mut merged_parameters = self.merged_parameters(Some(script), parameters)?;
        let mut job_result = JobResult::try_from((self, script, false))?;
        let id = job_result.id.clone();

        let directory = default_job_results_location()?.join(&job_result.id);
        fs::create_dir_all(&directory).map_err(|e| format!("Failed to create job result directory: {}", e))?;

        job_result.save()?;

        tokio::spawn(async move {
            if let Err(e) = Job::execute_job_result(&mut job_result, &directory, &mut merged_parameters) {
                eprintln!("Job execution failed: {}", e);
            }
        });

        Ok(id)
    }

    pub fn execute_job_result(
        job_result: &mut JobResult,
        directory: &PathBuf,
        parameters: &mut HashMap<String, ScriptParameterType>,
    ) -> Result<(), String> {
        let mut is_success = true;

        while job_result.finished_at.is_none() {
            job_result.start_step()?;

            let current_step = job_result
                .get_current_step_mut()
                .ok_or("No current step found")?
                .clone();

            let step_name = current_step.name.clone();

            if let Err(e) = current_step.execute(parameters, directory, &step_name, job_result) {
                let message = format!("Error in step {}: {}", step_name, e);
                job_result.add_log(LogLevel::Error, message.clone());
                job_result.finish_step(false)?;
                is_success = false;

                if job_result.dry_run {
                    return Err(message);
                }
                break;
            }

            if let Err(e) = job_result.finish_step(true) {
                let message = format!("Error finishing step {}: {}", step_name, e);
                job_result.add_log(LogLevel::Error, message.clone());
                is_success = false;

                if job_result.dry_run {
                    return Err(message);
                }
                break;
            }
        }

        job_result.is_success = is_success;
        job_result.save()?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_json_schema() -> Result<serde_json::Value, String> {
        serde_json::to_value(schema_for!(Job)).map_err(|e| format!("Failed to generate JSON schema: {}", e))
    }

    /// Checks parameters against script parameters. Respects default values.
    /// If script has a parameter without a default value, it must be provided.
    /// If script has a parameter with a default value, it can be omitted.
    pub fn validate_parameters(&self, script: Option<&Script>) -> Result<(), String> {
        let script = self.get_script(script)?;
        let mut missing_parameters = Vec::new();

        for parameter in &script.parameters {
            let is_parameter_defined = self.parameters.iter().any(|p| p.name == parameter.name);

            if !is_parameter_defined && parameter.default.is_none() && parameter.required {
                missing_parameters.push(parameter.name.clone());
            }
        }

        if !missing_parameters.is_empty() {
            return Err(format!(
                "Missing required parameters: {}",
                missing_parameters.join(", ")
            ));
        }

        Ok(())
    }

    pub fn merged_parameters(
        &self,
        script: Option<&Script>,
        parameters: HashMap<String, ScriptParameterType>,
    ) -> Result<HashMap<String, ScriptParameterType>, String> {
        let script = self.get_script(script)?;
        let mut merged_parameters = HashMap::new();

        for parameter in &script.parameters {
            let value = self.resolve_parameter_value(parameter, &parameters)?;

            if let Some(value) = value {
                merged_parameters.insert(format!("parameters.{}", parameter.name), value);
            }
        }

        Ok(merged_parameters)
    }

    fn resolve_parameter_value(
        &self,
        script_parameter: &ScriptParameter,
        provided_parameters: &HashMap<String, ScriptParameterType>,
    ) -> Result<Option<ScriptParameterType>, String> {
        let job_parameter = self.parameters.iter().find(|p| p.name == script_parameter.name);

        Ok(match job_parameter {
            Some(job_param) => provided_parameters
                .get(&job_param.name)
                .cloned()
                .or_else(|| job_param.default.clone()),
            None => script_parameter.default.clone(),
        })
    }

    pub fn validate(
        &self,
        script: Option<&Script>,
        parameters: HashMap<String, ScriptParameterType>,
    ) -> Result<(), String> {
        self.validate_parameters(script)?;

        let script = self.get_script(script)?;
        let mut merged_parameters = self.merged_parameters(Some(&script), parameters)?;

        let mut job_result =
            JobResult::try_from((self, &script, true)).map_err(|e| format!("Failed to create job result: {}", e))?;

        let directory = PathBuf::from("tmp");

        Job::execute_job_result(&mut job_result, &directory, &mut merged_parameters)
    }
}

impl TryFrom<PathBuf> for Job {
    type Error = String;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(&path).map_err(|e| format!("Failed to open job file {}: {}", path.display(), e))?;

        let reader = BufReader::new(file);
        serde_yaml::from_reader(reader).map_err(|e| format!("Failed to parse job YAML from {}: {}", path.display(), e))
    }
}

impl From<&Script> for Job {
    fn from(script: &Script) -> Self {
        let parameters = script
            .parameters
            .iter()
            .map(|p| JobParameterDefinition {
                name: p.name.clone(),
                default: p.default.clone(),
            })
            .collect();

        Self {
            id: "id".to_string(),
            name: script.name.clone(),
            parameters,
            triggers: vec![
                TriggerType::Manual(Default::default()),
                TriggerType::Github(Default::default()),
            ],
            script_id: script.id.clone(),
            read_only: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::script::models::YamlScriptStep;

    use super::*;

    mod job_parameters_test {
        use super::*;

        // Helper functions
        fn create_test_job(parameters: Vec<JobParameterDefinition>) -> Job {
            Job {
                id: "test-job".to_string(),
                name: "Test Job".to_string(),
                parameters,
                triggers: vec![],
                script_id: "test-script".to_string(),
                read_only: false,
            }
        }

        fn create_test_script(parameters: Vec<ScriptParameter>, steps: Vec<YamlScriptStep>) -> Script {
            Script {
                id: "test-script".to_string(),
                name: "Test Script".to_string(),
                parameters,
                steps,
            }
        }

        fn create_test_script_parameter(
            name: &str,
            required: bool,
            default: Option<ScriptParameterType>,
        ) -> ScriptParameter {
            ScriptParameter {
                name: name.to_string(),
                description: format!("Test parameter {}", name),
                required,
                default,
            }
        }

        fn create_test_job_parameter(name: &str, default: Option<ScriptParameterType>) -> JobParameterDefinition {
            JobParameterDefinition {
                name: name.to_string(),
                default,
            }
        }

        #[test]
        fn test_empty_parameters() {
            let job = create_test_job(vec![]);
            let script = create_test_script(vec![], vec![]);

            assert!(job.validate_parameters(Some(&script)).is_ok());
        }

        #[test]
        fn test_required_parameter_provided() {
            let job = create_test_job(vec![create_test_job_parameter(
                "param1",
                Some(ScriptParameterType::String("value1".to_string())),
            )]);

            let script = create_test_script(vec![create_test_script_parameter("param1", true, None)], vec![]);

            assert!(job.validate_parameters(Some(&script)).is_ok());
        }

        #[test]
        fn test_required_parameter_missing() {
            let job = create_test_job(vec![]);
            let script = create_test_script(vec![create_test_script_parameter("param1", true, None)], vec![]);

            let result = job.validate_parameters(Some(&script));
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "Missing required parameters: param1");
        }

        #[test]
        fn test_optional_parameter_with_default() {
            let job = create_test_job(vec![]);
            let script = create_test_script(
                vec![create_test_script_parameter(
                    "param1",
                    true,
                    Some(ScriptParameterType::String("default".to_string())),
                )],
                vec![],
            );

            assert!(job.validate_parameters(Some(&script)).is_ok());
        }

        #[test]
        fn test_multiple_parameters_mixed() {
            let job = create_test_job(vec![create_test_job_parameter(
                "param1",
                Some(ScriptParameterType::String("value1".to_string())),
            )]);

            let script = create_test_script(
                vec![
                    create_test_script_parameter("param1", true, None),
                    create_test_script_parameter("param2", true, Some(ScriptParameterType::Number(42))),
                    create_test_script_parameter("param3", false, None),
                ],
                vec![],
            );

            assert!(job.validate_parameters(Some(&script)).is_ok());
        }

        #[test]
        fn test_multiple_required_parameters_missing() {
            let job = create_test_job(vec![]);
            let script = create_test_script(
                vec![
                    create_test_script_parameter("param1", true, None),
                    create_test_script_parameter("param2", true, None),
                ],
                vec![],
            );

            let result = job.validate_parameters(Some(&script));
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "Missing required parameters: param1, param2");
        }
    }
}
