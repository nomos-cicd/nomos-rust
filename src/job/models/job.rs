use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
};

use crate::{
    job::{
        execution::JobExecutor,
        models::{JobParameterDefinition, JobResult},
        utils::default_jobs_location,
    },
    script::{models::Script, ScriptParameter, ScriptParameterType},
};

use super::trigger::TriggerType;

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
                        result.add_log(crate::log::LogLevel::Info, format!("Updated job {}", self.id));
                    }
                } else if let Some(result) = job_result {
                    result.add_log(crate::log::LogLevel::Info, format!("No changes in job {}", self.id));
                }
            }
            None => {
                self.save()?;
                if let Some(result) = job_result {
                    result.add_log(crate::log::LogLevel::Info, format!("Created job {}", self.id));
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
        JobExecutor::execute_with_script(self, parameters, &script)
    }

    pub fn validate(
        &self,
        script: Option<&Script>,
        parameters: HashMap<String, ScriptParameterType>,
    ) -> Result<(), String> {
        self.validate_parameters(script)?;
        let script = self.get_script(script)?;
        JobExecutor::validate(self, &script, parameters)
    }

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

    #[allow(dead_code)]
    pub fn get_json_schema() -> Result<serde_json::Value, String> {
        serde_json::to_value(schema_for!(Job)).map_err(|e| format!("Failed to generate JSON schema: {}", e))
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
