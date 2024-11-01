use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use sysinfo::System;
use tokio::{
    sync::Mutex,
    task::{self},
};

use crate::{
    job::models::{Job, JobResult},
    script::{models::Script, ScriptExecutionContext, ScriptExecutor, ScriptParameterType},
    utils::get_process_recursive,
};

#[derive(Debug, Clone)]
pub struct JobExecutor {
    handles: Arc<Mutex<HashMap<String, task::AbortHandle>>>,
}

impl Default for JobExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl JobExecutor {
    pub fn new() -> Self {
        JobExecutor {
            handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn execute_with_script(
        &self,
        job: &Job,
        parameters: HashMap<String, ScriptParameterType>,
        script: &Script,
    ) -> Result<String, String> {
        job.validate_parameters(Some(script))?;

        let mut merged_parameters = job.merged_parameters(Some(script), parameters.clone())?;
        let job_result = JobResult::try_from((job, script, false))?;
        let id = job_result.id.clone();
        let cloned_id = id.clone();
        let other_id = id.clone();

        let directory = crate::job::utils::default_job_results_location()?.join(&job_result.id);
        fs::create_dir_all(&directory).map_err(|e| format!("Failed to create job result directory: {}", e))?;

        job_result.save()?;

        let mut job_result_clone = job_result.clone();
        let handle = task::spawn(async move {
            let _res =
                Self::execute_job_result_internal(&mut job_result_clone, &directory, &mut merged_parameters).await;
        });
        let abort_handle = handle.abort_handle();
        task::spawn(async move {
            match handle.await {
                Ok(_) => {}
                Err(e) => {
                    if e.is_cancelled() {
                        let message = format!("Cancelled job {}: {}", other_id, e);
                        match JobResult::get(other_id.as_str()) {
                            Ok(Some(mut job_result)) => {
                                job_result.add_log(crate::log::LogLevel::Error, message.clone());
                                let s = System::new_all();
                                for child_process in &job_result.child_process_ids {
                                    let mut processes = get_process_recursive(*child_process);
                                    processes.reverse(); // Kill child processes first
                                    eprintln!("Killing processes with PID {}", child_process);
                                    for process in processes {
                                        if let Some(process) = s.process(process) {
                                            job_result.add_log(
                                                crate::log::LogLevel::Info,
                                                format!("Killing process with PID {}", process.pid()),
                                            );
                                            process.kill();
                                        } else {
                                            eprintln!("Process with PID {} not found", process);
                                        }
                                    }
                                }
                                match job_result.finish_step(false) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        eprintln!("Failed to finish step: {}", e);
                                    }
                                }
                                job_result.child_process_ids.clear();
                                job_result.is_success = false;
                                match job_result.save() {
                                    Ok(_) => {}
                                    Err(e) => {
                                        eprintln!("Failed to save job result: {}", e);
                                    }
                                }
                            }
                            Ok(None) => {
                                eprintln!("{}", message);
                            }
                            Err(e) => {
                                eprintln!("Failed to get job result: {}", e);
                            }
                        }
                    }
                }
            }
        });

        self.handles.lock().await.insert(id, abort_handle);

        Ok(cloned_id)
    }

    async fn execute_job_result_internal(
        job_result: &mut JobResult,
        directory: &Path,
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

            let mut context = ScriptExecutionContext {
                parameters,
                directory,
                step_name: &step_name,
                job_result,
            };

            if let Err(e) = current_step.execute(&mut context).await {
                let message = format!("Error in step {}: {}", step_name, e);
                job_result.add_log(crate::log::LogLevel::Error, message.clone());
                job_result.finish_step(false)?;
                is_success = false;

                if job_result.dry_run {
                    return Err(message);
                }
                break;
            }

            if let Err(e) = job_result.finish_step(true) {
                let message = format!("Error finishing step {}: {}", step_name, e);
                job_result.add_log(crate::log::LogLevel::Error, message.clone());
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

    pub async fn validate(
        &self,
        job: &Job,
        script: &Script,
        parameters: HashMap<String, ScriptParameterType>,
    ) -> Result<(), String> {
        let mut merged_parameters = job.merged_parameters(Some(script), parameters)?;
        let mut job_result = JobResult::try_from((job, script, true))?;
        let directory = PathBuf::from("tmp");

        Self::execute_job_result_internal(&mut job_result, &directory, &mut merged_parameters).await
    }

    pub async fn stop_job(&self, id: &str) -> Result<(), String> {
        let mut handles = self.handles.lock().await;
        if let Some(handle) = handles.get(id) {
            handle.abort();
            handles.remove(id);
            Ok(())
        } else {
            Err(format!("Job {} not found", id))
        }
    }
}
