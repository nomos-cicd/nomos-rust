use std::{collections::HashMap, fs::File, io::BufReader, path::PathBuf};

use crate::script::{Script, ScriptExecutor, ScriptStep};

use chrono::{DateTime, Utc};
use serde::Deserialize;

pub struct ManualTriggerParameter {}

#[derive(Deserialize, Debug)]
pub struct GithubTriggerParameter {
    pub branch: String,
    pub events: Vec<String>,
    pub secret: String,
    pub url: String,
}

pub enum TriggerType {
    Manual(ManualTriggerParameter),
    Github(GithubTriggerParameter),
}

pub struct Trigger {
    pub value: TriggerType,
}

#[derive(Deserialize, Debug)]
pub struct JobParameterDefinition {
    pub name: String,
    pub default: Option<String>,
}

pub struct Job {
    pub id: String,
    pub name: String,
    pub parameters: Vec<JobParameterDefinition>,
    pub triggers: Vec<Trigger>,
    pub script_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

#[derive(Deserialize, Debug)]
pub struct YamlTrigger {
    #[serde(rename = "type")]
    pub type_: String,
    pub value: serde_yaml::Value,
}

#[derive(Deserialize, Debug)]
pub struct YamlJob {
    pub id: String,
    pub name: String,
    pub parameters: Vec<JobParameterDefinition>,
    pub triggers: Vec<YamlTrigger>,
    pub script_id: String,
}

impl TryFrom<YamlJob> for Job {
    type Error = String;

    fn try_from(value: YamlJob) -> Result<Self, Self::Error> {
        let mut job = Job::default();
        job.id = value.id;
        job.name = value.name;
        job.script_id = value.script_id;

        for parameter in value.parameters {
            job.parameters.push(parameter);
        }

        for trigger in value.triggers {
            match trigger.type_.as_str() {
                "manual" => {
                    job.triggers.push(Trigger {
                        value: TriggerType::Manual(ManualTriggerParameter {}),
                    });
                }
                "github" => {
                    let github_trigger: GithubTriggerParameter =
                        serde_yaml::from_value(trigger.value).expect("Could not parse trigger");
                    job.triggers.push(Trigger {
                        value: TriggerType::Github(github_trigger),
                    });
                }
                _ => {
                    return Err(format!("Unknown trigger type: {}", trigger.type_));
                }
            }
        }

        Ok(job)
    }
}

impl TryFrom<PathBuf> for YamlJob {
    type Error = String;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(&path).expect("Could not open file");
        let reader = BufReader::new(file);
        let yaml: serde_yaml::Value = serde_yaml::from_reader(reader).expect("Could not read file");
        let yaml_job: YamlJob = serde_yaml::from_value(yaml).expect("Could not parse YAML");
        Ok(yaml_job)
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
    pub fn execute(&self, parameters: HashMap<String, String>) -> JobResult {
        let script = Script::get(&self.script_id).expect("Could not get script");
        self.execute_with_script(parameters, &script)
    }

    pub fn execute_with_script(
        &self,
        parameters: HashMap<String, String>,
        script: &Script,
    ) -> JobResult {
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
            panic!(
                "Missing parameters: {}",
                missing_parameters.join(", ")
            );
        }

        let mut job_result = Box::new(JobResult::from((self, script)));
        let directory = default_job_results_location().join(&job_result.id);
        std::fs::create_dir_all(&directory).expect("Could not create job_results directory");

        job_result.start_step();
        while !job_result.finished_at.is_some() {
            let current_step = job_result.current_step.as_ref().unwrap();
            let result = current_step.execute(merged_parameters.clone(), directory.clone());
            if result.is_err() {
                job_result.finish_step(false);
                break;
            }
            job_result.finish_step(true);
        }

        *job_result
    }
}

pub fn default_job_results_location() -> PathBuf {
    let path = if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").expect("Could not get APPDATA");
        PathBuf::from(appdata).join("nomos").join("job_results")
    } else {
        PathBuf::from("/var/lib/nomos/job_results")
    };
    std::fs::create_dir_all(&path).expect("Could not create job_results directory");
    path
}
