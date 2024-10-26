use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufReader, path::PathBuf};

use crate::{
    job::next_job_result_id,
    log::{JobLogger, LogLevel},
    script::models::{Script, ScriptStep},
};

use super::{default_job_results_location, Job};

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
    #[serde(skip)]
    pub dry_run: bool,
}

impl JobResult {
    pub fn get_current_step_mut(&mut self) -> Option<&mut ScriptStep> {
        if let Some(ref current_step_name) = self.current_step_name {
            self.steps.iter_mut().find(|step| step.name == *current_step_name)
        } else {
            None
        }
    }

    pub fn start_step(&mut self) -> Result<(), String> {
        if let Some(_current_step) = &self.current_step_name {
            let current_step = self.get_current_step_mut();
            if let Some(current_step) = current_step {
                current_step.start();
                self.save()?;
            } else {
                return Err("No current step".to_string());
            }
        } else {
            return Err("No current step".to_string());
        }

        Ok(())
    }

    pub fn finish_step(&mut self, is_success: bool) -> Result<(), String> {
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
                self.save()?;
            }

            let index = self.steps.iter().position(|step| step.name == current_step_name);
            if let Some(index) = index {
                if index + 1 < self.steps.len() {
                    self.current_step_name = self.steps.get(index + 1).cloned().map(|step| step.name);
                    self.updated_at = now;
                } else {
                    let now: DateTime<Utc> = Utc::now();
                    self.updated_at = now;
                    self.finished_at = Some(now);
                }
                self.save()?;
            }
        } else {
            return Err("No current step".to_string());
        }

        Ok(())
    }

    pub fn add_log(&mut self, level: LogLevel, message: String) {
        eprintln!("{:?}: {}", level, message);
        if self.dry_run {
            return;
        }
        if let Some(current_step_name) = &self.current_step_name {
            let _ = self.logger.log(level, current_step_name, &message);
        }
    }

    pub fn get_all(job_id: Option<String>) -> Result<Vec<Self>, String> {
        let path = default_job_results_location()?;
        let mut job_results = Vec::new();
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let mut path = entry.path();
            path.push("result.yml");
            let job_result =
                JobResult::try_from(path.clone()).map_err(|e| format!("Path: {:?}, Error: {:?}", path, e))?;
            if let Some(job_id) = job_id.clone() {
                if job_result.job_id == job_id {
                    job_results.push(job_result);
                }
            } else {
                job_results.push(job_result);
            }
        }
        job_results.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(job_results)
    }

    pub fn get(id: &str) -> Result<Option<Self>, String> {
        let path = default_job_results_location()?.join(id).join("result.yml");
        if path.exists() {
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            serde_yaml::from_str(&content).map_err(|e| e.to_string())
        } else {
            Ok(None)
        }
    }

    pub fn save(&self) -> Result<(), String> {
        if self.dry_run {
            return Ok(());
        }
        let path = default_job_results_location()?.join(&self.id).join("result.yml");
        let file = File::create(path).map_err(|e| e.to_string())?;
        serde_yaml::to_writer(file, self).map_err(|e| e.to_string())
    }

    #[allow(dead_code)]
    pub async fn wait_for_completion(id: &str) -> Result<Self, String> {
        let job_result = JobResult::get(id)?;
        if job_result.is_none() {
            return Err("Job result not found".to_string());
        }
        let mut job_result = job_result.unwrap();
        while job_result.finished_at.is_none() {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let new_job_result = JobResult::get(id)?;
            if new_job_result.is_none() {
                return Err("Job result not found".to_string());
            }
            job_result = new_job_result.unwrap();
        }
        Ok(job_result)
    }

    /// Will fix later
    pub fn create_dummy() -> Self {
        JobResult {
            id: "dummy".to_string(),
            job_id: "dummy".to_string(),
            is_success: false,
            steps: vec![],
            current_step_name: None,
            started_at: Utc::now(),
            updated_at: Utc::now(),
            finished_at: None,
            logger: JobLogger::new("dummy".to_string(), "dummy".to_string()).unwrap(),
            dry_run: false,
        }
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

impl TryFrom<&Job> for JobResult {
    type Error = String;
    fn try_from(job: &Job) -> Result<Self, Self::Error> {
        let id = next_job_result_id().map_err(|e| e.to_string())?;
        let script = Script::get(&job.script_id)?;
        if script.is_none() {
            return Err(format!("Script with id '{}' not found", job.script_id));
        }
        let script = script.unwrap();
        let steps: Vec<ScriptStep> = script.steps.iter().map(|step| ScriptStep::from(step)).collect();
        let logger = JobLogger::new(job.id.clone(), id.clone())?;
        Ok(JobResult {
            id,
            job_id: job.id.clone(),
            steps: steps.clone(),
            current_step_name: steps.first().map(|step| step.name.clone()),
            is_success: false,
            started_at: Utc::now(),
            updated_at: Utc::now(),
            finished_at: None,
            logger,
            dry_run: false,
        })
    }
}

impl TryFrom<(&Job, &Script, bool)> for JobResult {
    type Error = String;
    fn try_from((job, script, dry_mode): (&Job, &Script, bool)) -> Result<Self, Self::Error> {
        let id = if !dry_mode {
            next_job_result_id().unwrap()
        } else {
            "dry_run".to_string()
        };
        let steps: Vec<ScriptStep> = script.steps.iter().map(ScriptStep::from).collect();
        let logger = JobLogger::new(job.id.clone(), id.clone())?;
        Ok(JobResult {
            id,
            job_id: job.id.clone(),
            steps: steps.clone(),
            current_step_name: steps.first().map(|step| step.name.clone()),
            dry_run: dry_mode,
            is_success: false,
            started_at: Utc::now(),
            updated_at: Utc::now(),
            finished_at: None,
            logger,
        })
    }
}
