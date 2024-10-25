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
                self.save();
            }
        } else {
            panic!("No current step");
        }
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

    pub fn get_all() -> Result<Vec<Self>, String> {
        let path = default_job_results_location();
        let mut job_results = Vec::new();
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string()).unwrap() {
            let entry = entry.map_err(|e| e.to_string()).unwrap();
            let mut path = entry.path();
            path.push("result.yml");
            let job_result =
                JobResult::try_from(path.clone()).map_err(|e| format!("Path: {:?}, Error: {:?}", path, e))?;
            job_results.push(job_result);
        }
        job_results.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(job_results)
    }

    pub fn get(id: &str) -> Option<Self> {
        let path = default_job_results_location().join(id).join("result.yml");
        if path.exists() {
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string()).ok()?;
            serde_yaml::from_str(&content).map_err(|e| e.to_string()).ok()
        } else {
            None
        }
    }

    pub fn save(&self) {
        if self.dry_run {
            return;
        }
        let path = default_job_results_location().join(&self.id).join("result.yml");
        let file = File::create(path).map_err(|e| e.to_string()).unwrap();
        serde_yaml::to_writer(file, self).map_err(|e| e.to_string()).unwrap();
    }

    #[allow(dead_code)]
    pub async fn wait_for_completion(id: &str) -> Result<Self, String> {
        let mut job_result = JobResult::get(id).ok_or("Could not get job result")?;
        while job_result.finished_at.is_none() {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            job_result = JobResult::get(id).ok_or("Could not get job result")?;
        }
        Ok(job_result)
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

impl Default for JobResult {
    fn default() -> Self {
        let id = next_job_result_id().unwrap();
        let logger = JobLogger::new(id.clone(), id.clone()).unwrap();

        eprintln!("JobResult ID: {}", id);
        JobResult {
            id,
            job_id: String::new(),
            is_success: false,
            steps: vec![],
            current_step_name: None,
            started_at: Utc::now(),
            updated_at: Utc::now(),
            finished_at: None,
            logger,
            dry_run: false,
        }
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

impl From<(&Job, &Script, bool)> for JobResult {
    fn from((job, script, dry_mode): (&Job, &Script, bool)) -> Self {
        let id = if !dry_mode {
            next_job_result_id().unwrap()
        } else {
            "dry_run".to_string()
        };
        let steps: Vec<ScriptStep> = script.steps.iter().map(ScriptStep::from).collect();
        JobResult {
            id,
            job_id: job.id.clone(),
            steps: steps.clone(),
            current_step_name: steps.first().map(|step| step.name.clone()),
            dry_run: dry_mode,
            ..Default::default()
        }
    }
}
