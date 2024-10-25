mod job_result;
mod trigger;
mod utils;

pub use job_result::JobResult;
pub use trigger::GithubPayload;
pub use trigger::TriggerType;
pub use utils::{default_job_results_location, default_jobs_location, next_job_result_id};

use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, io::BufReader, path::PathBuf};

use crate::{
    log::LogLevel,
    script::{models::Script, ScriptExecutor, ScriptParameterType},
};

#[derive(Deserialize, Serialize, PartialEq, Clone, JsonSchema, Debug)]
pub struct JobParameterDefinition {
    pub name: String,
    pub default: Option<ScriptParameterType>,
}

#[derive(Deserialize, Serialize, Default, JsonSchema, Debug)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub parameters: Vec<JobParameterDefinition>,
    pub triggers: Vec<TriggerType>,
    pub script_id: String,
    pub read_only: bool,
}

impl Job {
    pub fn get(id: &str) -> Option<Self> {
        let path = default_jobs_location().join(format!("{}.yml", id));
        if path.exists() {
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string()).ok()?;
            serde_yaml::from_str(&content).map_err(|e| e.to_string()).ok()
        } else {
            None
        }
    }

    pub fn get_all() -> Result<Vec<Self>, String> {
        let path = default_jobs_location();
        let mut jobs = Vec::new();
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let job = Job::try_from(path).map_err(|e| e.to_string())?;
            jobs.push(job);
        }
        Ok(jobs)
    }

    pub fn sync(&self, job_result: Option<&mut JobResult>) -> Result<(), String> {
        self.validate(None, Default::default())?;

        let existing_job = Job::get(self.id.as_str());
        if let Some(existing_job) = existing_job {
            if existing_job.name != self.name
                || existing_job.parameters != self.parameters
                || existing_job.triggers != self.triggers
                || existing_job.script_id != self.script_id
            {
                self.save();
                if let Some(job_result) = job_result {
                    job_result.add_log(LogLevel::Info, format!("Updated job {:?}", self.id))
                }
            } else if let Some(job_result) = job_result {
                job_result.add_log(LogLevel::Info, format!("No changes in job {:?}", self.id))
            }
        } else {
            self.save();
            if let Some(job_result) = job_result {
                job_result.add_log(LogLevel::Info, format!("Created job {:?}", self.id))
            }
        }

        Ok(())
    }

    fn save(&self) {
        let path = default_jobs_location().join(format!("{}.yml", self.id));
        let file = File::create(path).map_err(|e| e.to_string()).unwrap();
        serde_yaml::to_writer(file, self).map_err(|e| e.to_string()).unwrap();
    }

    pub fn delete(&self) {
        let path = PathBuf::from("jobs").join(format!("{}.yml", self.id));
        std::fs::remove_file(path).map_err(|e| e.to_string()).unwrap();
    }

    pub fn execute(&self, parameters: HashMap<String, ScriptParameterType>) -> Result<String, String> {
        let script = Script::get(&self.script_id).ok_or("Could not get script")?;
        self.execute_with_script(parameters, &script)
    }

    pub fn execute_with_script(
        &self,
        parameters: HashMap<String, ScriptParameterType>,
        script: &Script,
    ) -> Result<String, String> {
        self.validate(Some(script), parameters.clone())?;

        let mut merged_parameters = self.merged_parameters(Some(script), parameters.clone())?;

        let mut job_result = JobResult::from((self, script, false));
        let id = job_result.id.clone();
        let directory = default_job_results_location().join(&job_result.id);
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        job_result.save();

        tokio::spawn(async move {
            let _ = Job::execute_job_result(&mut job_result, directory, &mut merged_parameters);
        });

        Ok(id)
    }

    /// parameters should be prepared by the caller
    pub fn execute_job_result(
        job_result: &mut JobResult,
        directory: PathBuf,
        parameters: &mut HashMap<String, ScriptParameterType>,
    ) -> Result<(), String> {
        let mut is_success = true;
        while job_result.finished_at.is_none() {
            let _ = &job_result.start_step();

            // Clone `current_step` to avoid immutable borrow on `job_result`
            let current_step = job_result.get_current_step_mut().unwrap().clone();
            let step_name = current_step.name.clone();

            // Mutable borrow of `job_result` is now safe
            let result = current_step.execute(parameters, directory.clone(), step_name.as_str(), job_result);

            if result.is_err() {
                let message = format!("Error in step {}: {:?}", step_name, result.err().unwrap());
                job_result.add_log(LogLevel::Error, message.clone());
                job_result.finish_step(false);
                is_success = false;
                if job_result.dry_run {
                    return Err(message.clone());
                }
                break;
            }
            job_result.finish_step(true);
        }

        job_result.is_success = is_success;
        job_result.save();

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_json_schema() -> Result<serde_json::Value, String> {
        let schema = schema_for!(Job);
        serde_json::to_value(schema).map_err(|e| e.to_string())
    }

    /// Checks parameters against script parameters. Respects default values.
    /// If script has a parameter without a default value, it must be provided.
    /// If script has a parameter with a default value, it can be omitted.
    pub fn validate_parameters(&self, script: Option<&Script>) -> Result<(), String> {
        let script_opt: Option<Script> = match script {
            Some(script) => Some(script.clone()),
            None => {
                let self_script = Script::get(&self.script_id).ok_or("Could not get script")?;
                Some(self_script.clone())
            }
        };
        let script = script_opt.unwrap();

        let mut missing_parameters = vec![];
        for parameter in &script.parameters {
            if !self.parameters.iter().any(|p| p.name == parameter.name)
                && parameter.default.is_none()
                && parameter.required
            {
                missing_parameters.push(parameter.name.clone());
            }
        }

        if !missing_parameters.is_empty() {
            return Err(format!("Missing parameters: {}", missing_parameters.join(", ")));
        }

        Ok(())
    }

    /// Merges script and job parameters. Respects default values.
    /// Also adds 'parameters.' prefix to each parameter.
    pub fn merged_parameters(
        &self,
        script: Option<&Script>,
        parameters: HashMap<String, ScriptParameterType>,
    ) -> Result<HashMap<String, ScriptParameterType>, String> {
        let script_opt: Option<Script> = match script {
            Some(script) => Some(script.clone()),
            None => {
                let self_script = Script::get(&self.script_id).ok_or("Could not get script")?;
                Some(self_script.clone())
            }
        };
        let script = script_opt.unwrap();

        let mut merged_parameters = HashMap::new();
        for parameter in &script.parameters {
            let job_parameter = self.parameters.iter().find(|p| p.name == parameter.name);
            let value = match job_parameter {
                Some(job_parameter) => {
                    let value = parameters.get(&job_parameter.name).cloned();
                    if value.is_none() {
                        job_parameter.default.clone()
                    } else {
                        value
                    }
                }
                None => parameter.default.clone(),
            };

            if let Some(value) = value {
                merged_parameters.insert(format!("parameters.{}", parameter.name), value);
            }
        }

        Ok(merged_parameters)
    }

    pub fn validate(
        &self,
        script: Option<&Script>,
        parameters: HashMap<String, ScriptParameterType>,
    ) -> Result<(), String> {
        self.validate_parameters(script)?;
        let mut merged_parameters = self.merged_parameters(script, parameters)?;
        let script_opt: Option<Script> = match script {
            Some(script) => Some(script.clone()),
            None => {
                let self_script = Script::get(&self.script_id).ok_or("Could not get script")?;
                Some(self_script.clone())
            }
        };
        let script = script_opt.unwrap();

        let directory = PathBuf::from("tmp");
        Job::execute_job_result(
            &mut JobResult::from((self, &script, true)),
            directory,
            &mut merged_parameters,
        )
    }
}

impl TryFrom<PathBuf> for Job {
    type Error = String;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(path).map_err(|_| "Could not open file")?;
        let reader = BufReader::new(file);
        serde_yaml::from_reader(reader).map_err(|e| e.to_string())
    }
}

impl From<&Script> for Job {
    fn from(script: &Script) -> Self {
        let parameters = script.parameters.iter().map(|p| JobParameterDefinition {
            name: p.name.clone(),
            default: p.default.clone(),
        });

        Job {
            id: "id".to_string(),
            name: script.name.clone(),
            parameters: parameters.collect(),
            triggers: vec![
                TriggerType::Manual(Default::default()),
                TriggerType::Github(Default::default()),
            ],
            script_id: script.id.clone(),
            read_only: false,
        }
    }
}
