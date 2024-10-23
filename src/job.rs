use std::{collections::HashMap, fs::File, io::BufReader, path::PathBuf};

use crate::{
    log::{JobLogger, LogLevel},
    script::{Script, ScriptExecutor, ScriptParameterType, ScriptStep},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct ManualTriggerParameter {}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct GithubTriggerParameter {
    pub branch: String,
    pub events: Vec<String>,
    pub secret: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum TriggerType {
    #[serde(rename = "manual")]
    Manual(ManualTriggerParameter),
    #[serde(rename = "github")]
    Github(GithubTriggerParameter),
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub struct JobParameterDefinition {
    pub name: String,
    pub default: Option<ScriptParameterType>,
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub parameters: Vec<JobParameterDefinition>,
    pub triggers: Vec<TriggerType>,
    pub script_id: String,
    pub read_only: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobResult {
    pub id: String,
    pub job_id: String,
    pub is_success: bool,
    pub steps: Vec<ScriptStep>,
    pub current_step_name: Option<String>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub logger: JobLogger,
}

impl Default for JobResult {
    fn default() -> Self {
        let id = Uuid::new_v4().to_string();
        let logger = JobLogger::new(id.clone(), id.clone()).unwrap();
        JobResult {
            id,
            job_id: String::new(),
            is_success: true,
            steps: vec![],
            current_step_name: None,
            started_at: Utc::now(),
            updated_at: Utc::now(),
            finished_at: None,
            logger,
        }
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

impl From<&Job> for JobResult {
    fn from(job: &Job) -> Self {
        let steps: Vec<ScriptStep> = Script::get(&job.script_id)
            .map(|script| script.steps.iter().map(ScriptStep::from).collect())
            .unwrap();
        JobResult {
            job_id: job.id.clone(),
            steps: steps.clone(),
            current_step_name: steps.first().map(|step| step.name.clone()),
            ..Default::default()
        }
    }
}

impl From<(&Job, &Script)> for JobResult {
    fn from((job, _script): (&Job, &Script)) -> Self {
        let steps: Vec<ScriptStep> = Script::get(&job.script_id)
            .map(|script| script.steps.iter().map(ScriptStep::from).collect())
            .unwrap();
        JobResult {
            job_id: job.id.clone(),
            steps: steps.clone(),
            current_step_name: steps.first().map(|step| step.name.clone()),
            ..Default::default()
        }
    }
}

impl JobResult {
    pub fn get_current_step_mut(&mut self) -> Option<&mut ScriptStep> {
        if let Some(ref current_step_name) = self.current_step_name {
            self.steps
                .iter_mut()
                .find(|step| step.name == *current_step_name)
        } else {
            None
        }
    }

    pub fn start_step(&mut self) {
        if let Some(_current_step) = &self.current_step_name {
            let current_step = self.get_current_step_mut();
            if let Some(current_step) = current_step {
                current_step.start();
                self.save();
            } else {
                panic!("No current step");
            }
        } else {
            panic!("No current step");
        }
    }

    pub fn finish_step(&mut self, is_success: bool) {
        let now: DateTime<Utc> = Utc::now();
        if let Some(current_step_name) = self.current_step_name.clone() {
            let current_step = self.get_current_step_mut();
            if let Some(current_step) = current_step {
                current_step.finish(is_success);
            } else {
                panic!("No current step");
            }
            if !is_success {
                self.is_success = false;
                self.updated_at = now;
                self.finished_at = Some(now);
                self.save();
                return;
            }

            let index = self
                .steps
                .iter()
                .position(|step| step.name == current_step_name);
            if let Some(index) = index {
                if index + 1 < self.steps.len() {
                    self.current_step_name =
                        self.steps.get(index + 1).cloned().map(|step| step.name);
                    self.updated_at = now;
                } else {
                    let now: DateTime<Utc> = Utc::now();
                    self.updated_at = now;
                    self.finished_at = Some(now);
                }
                self.save();
            }
        } else {
            panic!("No current step");
        }
    }

    pub fn add_log(&mut self, level: LogLevel, message: String) {
        eprintln!("{:?}: {}", level, message);
        if let Some(current_step_name) = &self.current_step_name {
            let _ = self.logger.log(level, current_step_name, &message);
        }
    }

    pub fn get_all() -> Vec<Self> {
        let path = default_job_results_location();
        let mut job_results = Vec::new();
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string()).unwrap() {
            let entry = entry.map_err(|e| e.to_string()).unwrap();
            let path = entry.path();
            let job_result = JobResult::try_from(path)
                .map_err(|e| e.to_string())
                .unwrap();
            job_results.push(job_result);
        }
        job_results
    }

    pub fn get(id: &str) -> Option<Self> {
        let path = default_job_results_location().join(id);
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .ok()?;
            serde_yaml::from_str(&content)
                .map_err(|e| e.to_string())
                .ok()
        } else {
            None
        }
    }

    pub fn delete(&self) {
        let path = default_job_results_location()
            .join(&self.id)
            .join("result.yml");
        std::fs::remove_file(path)
            .map_err(|e| e.to_string())
            .unwrap();
    }

    pub fn save(&self) {
        let path = default_job_results_location()
            .join(&self.id)
            .join("result.yml");
        let file = File::create(path).map_err(|e| e.to_string()).unwrap();
        serde_yaml::to_writer(file, self)
            .map_err(|e| e.to_string())
            .unwrap();
    }
}

impl TryFrom<PathBuf> for JobResult {
    type Error = String;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(path).map_err(|_| "Could not open file")?;
        let reader = BufReader::new(file);
        serde_yaml::from_reader(reader).map_err(|e| e.to_string())
    }
}

impl Job {
    pub fn get(id: &str) -> Option<Self> {
        let path = default_jobs_location().join(format!("{}.yml", id));
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .ok()?;
            serde_yaml::from_str(&content)
                .map_err(|e| e.to_string())
                .ok()
        } else {
            None
        }
    }

    pub fn get_all() -> Vec<Self> {
        let path = default_jobs_location();
        let mut jobs = Vec::new();
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string()).unwrap() {
            let entry = entry.map_err(|e| e.to_string()).unwrap();
            let path = entry.path();
            let job = Job::try_from(path).map_err(|e| e.to_string()).unwrap();
            jobs.push(job);
        }
        jobs
    }

    pub fn sync(&self, job_result: Option<&mut JobResult>) {
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
    }

    fn save(&self) {
        let path = default_jobs_location().join(format!("{}.yml", self.id));
        let file = File::create(path).map_err(|e| e.to_string()).unwrap();
        serde_yaml::to_writer(file, self)
            .map_err(|e| e.to_string())
            .unwrap();
    }

    pub fn delete(&self) {
        let path = PathBuf::from("jobs").join(format!("{}.yml", self.id));
        std::fs::remove_file(path)
            .map_err(|e| e.to_string())
            .unwrap();
    }

    pub fn execute(
        &self,
        parameters: HashMap<String, ScriptParameterType>,
    ) -> Result<JobResult, String> {
        let script = Script::get(&self.script_id).ok_or("Could not get script")?;
        self.execute_with_script(parameters, &script)
    }

    pub fn execute_with_script(
        &self,
        parameters: HashMap<String, ScriptParameterType>,
        script: &Script,
    ) -> Result<JobResult, String> {
        let mut merged_parameters = parameters.clone();
        for parameter in &self.parameters {
            if !parameters.contains_key(&parameter.name) {
                if let Some(default) = &parameter.default {
                    merged_parameters.insert(parameter.name.clone(), default.clone());
                }
            }
        }

        let mut missing_parameters = vec![];
        for parameter in &self.parameters {
            if !merged_parameters.contains_key(&parameter.name) {
                missing_parameters.push(parameter.name.clone());
            }
        }
        if !missing_parameters.is_empty() {
            panic!("Missing parameters: {}", missing_parameters.join(", "));
        }

        // Add '$parameters.' to each parameter
        let mut merged_parameters_with_prefix = HashMap::new();
        for (key, value) in merged_parameters.clone() {
            merged_parameters_with_prefix.insert(format!("$parameters.{}", key), value);
        }

        let mut job_result = JobResult::from((self, script));
        let directory = default_job_results_location().join(&job_result.id);
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;

        let _ = &job_result.start_step();
        while job_result.finished_at.is_none() {
            // Clone `current_step` to avoid immutable borrow on `job_result`
            let current_step = job_result.get_current_step_mut().unwrap().clone();
            let step_name = current_step.name.clone();

            // Mutable borrow of `job_result` is now safe
            let result = current_step.execute(
                &mut merged_parameters_with_prefix,
                directory.clone(),
                step_name.as_str(),
                &mut job_result,
            );

            if result.is_err() {
                job_result.finish_step(false);
                break;
            }
            job_result.finish_step(true);
        }

        Ok(job_result)
    }
}

pub fn default_job_results_location() -> PathBuf {
    let path = if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").map_err(|e| e.to_string()).unwrap();
        PathBuf::from(appdata).join("nomos").join("job_results")
    } else {
        PathBuf::from("/var/lib/nomos/job_results")
    };
    std::fs::create_dir_all(&path)
        .map_err(|e| e.to_string())
        .unwrap();
    path
}

pub fn default_jobs_location() -> PathBuf {
    let path = if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").map_err(|e| e.to_string()).unwrap();
        PathBuf::from(appdata).join("nomos").join("jobs")
    } else {
        PathBuf::from("/var/lib/nomos/jobs")
    };
    std::fs::create_dir_all(&path)
        .map_err(|e| e.to_string())
        .unwrap();
    path
}
