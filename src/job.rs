use std::{collections::HashMap, fs::File, io::BufReader, path::PathBuf};

use crate::script::{Script, ScriptExecutor, ScriptStep};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    pub default: Option<String>,
}

pub struct Job {
    pub id: String,
    pub name: String,
    pub parameters: Vec<JobParameterDefinition>,
    pub triggers: Vec<TriggerType>,
    pub script_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub read_only: bool,
}

#[derive(Debug)]
pub struct JobResult {
    pub id: String,
    pub job_id: String,
    pub is_success: bool,
    pub steps: Vec<ScriptStep>,
    pub current_step: Option<ScriptStep>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl Default for Job {
    fn default() -> Self {
        Job {
            id: String::new(),
            name: String::new(),
            parameters: vec![],
            triggers: vec![],
            script_id: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            read_only: false,
        }
    }
}

impl Default for JobResult {
    fn default() -> Self {
        JobResult {
            id: String::new(),
            job_id: String::new(),
            is_success: true,
            steps: vec![],
            current_step: None,
            started_at: Utc::now(),
            updated_at: Utc::now(),
            finished_at: None,
        }
    }
}

impl TryFrom<PathBuf> for Job {
    type Error = String;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(&path).map_err(|_| "Could not open file")?;
        let reader = BufReader::new(file);
        let yaml: serde_yaml::Value = serde_yaml::from_reader(reader).map_err(|e| e.to_string())?;
        let yaml_job: YamlJob = serde_yaml::from_value(yaml).map_err(|e| e.to_string())?;
        Job::try_from(yaml_job)
    }
}

impl From<&Job> for JobResult {
    fn from(job: &Job) -> Self {
        JobResult {
            job_id: job.id.clone(),
            steps: Script::get(&job.script_id)
                .unwrap()
                .steps
                .iter()
                .cloned()
                .collect(),
            current_step: Script::get(&job.script_id).unwrap().steps.get(0).cloned(),
            ..Default::default()
        }
    }
}

impl From<(&Job, &Script)> for JobResult {
    fn from((job, script): (&Job, &Script)) -> Self {
        JobResult {
            id: "1".to_string(),
            job_id: job.id.clone(),
            steps: script.steps.iter().cloned().collect(),
            current_step: script.steps.get(0).cloned(),
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct YamlJob {
    pub id: String,
    pub name: String,
    pub parameters: Vec<JobParameterDefinition>,
    pub triggers: Vec<TriggerType>,
    pub script_id: String,
    pub read_only: bool,
}

impl TryFrom<YamlJob> for Job {
    type Error = String;

    /// Reads as YamlJob and converts to Job. Primarily used before executing a job.
    fn try_from(value: YamlJob) -> Result<Self, Self::Error> {
        Ok(Job {
            id: value.id,
            name: value.name,
            parameters: value.parameters,
            triggers: value.triggers,
            script_id: value.script_id,
            read_only: value.read_only,
            ..Default::default()
        })
    }
}

impl TryFrom<PathBuf> for YamlJob {
    type Error = String;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(&path).map_err(|_| "Could not open file")?;
        let reader = BufReader::new(file);
        let yaml: serde_yaml::Value = serde_yaml::from_reader(reader).map_err(|e| e.to_string())?;
        let yaml_job: YamlJob = serde_yaml::from_value(yaml).map_err(|e| e.to_string())?;
        Ok(yaml_job)
    }
}

impl From<&Job> for YamlJob {
    fn from(job: &Job) -> Self {
        YamlJob {
            id: job.id.clone(),
            name: job.name.clone(),
            parameters: job.parameters.clone(),
            triggers: job.triggers.clone(),
            script_id: job.script_id.clone(),
            read_only: job.read_only,
        }
    }
}

impl JobResult {
    pub fn start_step(&mut self) {
        if let Some(current_step) = &self.current_step {
            self.current_step.as_mut().unwrap().start();
        } else {
            panic!("No current step");
        }
    }

    pub fn finish_step(&mut self, is_success: bool) {
        let now: DateTime<Utc> = Utc::now();
        if let Some(mut current_step) = self.current_step.as_mut() {
            current_step.finish(is_success);
            if !is_success {
                self.is_success = false;
                self.updated_at = now;
                self.finished_at = Some(now);
                return;
            }

            let index = self
                .steps
                .iter()
                .position(|step| step.name == current_step.name);
            if let Some(index) = index {
                if index + 1 < self.steps.len() {
                    self.current_step = self.steps.get(index + 1).cloned();
                    self.updated_at = now;
                } else {
                    let now: DateTime<Utc> = Utc::now();
                    self.updated_at = now;
                    self.finished_at = Some(now);
                }
            }
        } else {
            panic!("No current step");
        }
    }
}

impl Job {
    /// Reads as YamlJob and converts to Job. Primarily used before executing a job.
    pub fn get(id: &str) -> Option<Self> {
        let path = default_jobs_location().join(format!("{}.yaml", id));
        let yaml_job = YamlJob::try_from(path);
        if let Ok(yaml_job) = yaml_job {
            let job = Job::try_from(yaml_job);
            if let Ok(job) = job {
                Some(job)
            } else {
                None
            }
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

    pub fn sync(&self) {
        let existing_job = Job::get(self.id.as_str());
        if let Some(existing_job) = existing_job {
            if existing_job.name != self.name
                || existing_job.parameters != self.parameters
                || existing_job.triggers != self.triggers
                || existing_job.script_id != self.script_id
            {
                eprintln!("Updating job {:?}", existing_job.id);
                self.save();
            } else {
                eprintln!("Existing job {:?}", existing_job.id);
            }
        } else {
            eprintln!("New job {:?}", self.id);
            self.save();
        }
    }

    fn save(&self) {
        let path = default_jobs_location().join(format!("{}.yaml", self.id));
        let file = File::create(path).map_err(|e| e.to_string()).unwrap();
        serde_yaml::to_writer(file, &YamlJob::from(self))
            .map_err(|e| e.to_string())
            .unwrap();
    }

    pub fn delete(&self) {
        let path = PathBuf::from("jobs").join(format!("{}.yaml", self.id));
        std::fs::remove_file(&path)
            .map_err(|e| e.to_string())
            .unwrap();
    }

    pub fn execute(&self, parameters: HashMap<String, String>) -> Result<JobResult, String> {
        let script = Script::get(&self.script_id).ok_or("Could not get script")?;
        self.execute_with_script(parameters, &script)
    }

    pub fn execute_with_script(
        &self,
        parameters: HashMap<String, String>,
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

        let mut job_result = Box::new(JobResult::from((self, script)));
        let directory = default_job_results_location().join(&job_result.id);
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;

        job_result.start_step();
        while !job_result.finished_at.is_some() {
            let current_step = job_result.current_step.as_ref().unwrap();
            let result = current_step.execute(
                &mut merged_parameters_with_prefix,
                directory.clone(),
                current_step.name.as_str(),
            );
            if result.is_err() {
                job_result.finish_step(false);
                break;
            }
            job_result.finish_step(true);
        }

        Ok(*job_result)
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
