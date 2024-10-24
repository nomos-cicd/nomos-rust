use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARNING"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Log {
    pub level: LogLevel,
    pub message: String,
    pub step_name: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobLogger {
    log_filename: PathBuf,
    job_id: String,
    result_id: String,
}

impl JobLogger {
    pub fn new(job_id: String, result_id: String) -> Result<Self, String> {
        let log_path = get_log_file_path(&job_id, &result_id);

        // Create directory if it doesn't exist
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path.clone())
            .map_err(|e| e.to_string())?;

        Ok(JobLogger {
            log_filename: log_path.clone(),
            job_id,
            result_id,
        })
    }

    pub fn log(&mut self, level: LogLevel, step_name: &str, message: &str) -> Result<(), String> {
        let log = Log {
            level,
            message: message.to_string(),
            step_name: step_name.to_string(),
            timestamp: Utc::now(),
        };

        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.log_filename)
            .map_err(|e| e.to_string())?;

        writeln!(file, "{}", serde_json::to_string(&log).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn get_logs(&self) -> Result<Vec<Log>, String> {
        let path = get_log_file_path(&self.job_id, &self.result_id);
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;

        let logs = content
            .lines()
            .filter_map(|line| serde_json::from_str::<Log>(line).ok())
            .collect();

        Ok(logs)
    }
}

fn get_log_file_path(job_id: &str, result_id: &str) -> PathBuf {
    if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").unwrap();
        PathBuf::from(appdata)
            .join("nomos")
            .join("logs")
            .join(job_id)
            .join(format!("{}.log", result_id))
    } else {
        PathBuf::from("/var/lib/nomos/logs")
            .join(job_id)
            .join(format!("{}.log", result_id))
    }
}
