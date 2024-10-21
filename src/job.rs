use std::{fs::File, io::BufReader, path::PathBuf};

use crate::script::ScriptStep;

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

pub struct JobResult {
    pub id: String,
    pub job_id: String,
    pub is_finished: bool,
    pub is_success: bool,
    pub steps: Vec<ScriptStep>,
    pub current_step: Option<ScriptStep>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
