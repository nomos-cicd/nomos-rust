use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::select;
use tokio_util::sync::CancellationToken;
use tokio::sync::Mutex;

use crate::{
    job::models::{Job, JobResult},
    script::{models::Script, ScriptExecutionContext, ScriptExecutor, ScriptParameterType},
};

#[derive(Debug, Clone)]
pub struct JobExecutor {
    cancellation_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl JobExecutor {
    pub fn new() -> Self {
        JobExecutor {
            cancellation_tokens: Arc::new(Mutex::new(HashMap::new())),
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
        let mut job_result = JobResult::try_from((job, script, false))?;
        let id = job_result.id.clone();
        let cloned_id = id.clone();

        let directory = crate::job::utils::default_job_results_location()?.join(&job_result.id);
        fs::create_dir_all(&directory).map_err(|e| format!("Failed to create job result directory: {}", e))?;

        job_result.save()?;
        let cancellation_token = CancellationToken::new();

        // Store the cancellation token
        self.cancellation_tokens.lock().await.insert(id.clone(), cancellation_token.clone());

        let cloned_cancellation_token = cancellation_token.clone();
        let cloned_self = self.clone();
        tokio::spawn(async move {
            select! {
                _ = cloned_cancellation_token.cancelled() => {
                    job_result.add_log(crate::log::LogLevel::Info, "Job execution cancelled".to_string());
                    job_result.finish_step(false);
                    job_result.is_success = false;
                    job_result.save().unwrap();
                }

                result = cloned_self.execute_job_result(&mut job_result, &directory, &mut merged_parameters, &cloned_cancellation_token) => {
                    if let Err(e) = result {
                        eprintln!("Job execution failed: {}", e);
                    }
                }
            }

            // Remove the cancellation token after execution
            cloned_self.cancellation_tokens.lock().await.remove(&id);
        });

        Ok(cloned_id)
    }

    async fn execute_job_result(
        &self,
        job_result: &mut JobResult,
        directory: &Path,
        parameters: &mut HashMap<String, ScriptParameterType>,
        cancellation_token: &CancellationToken,
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
                cancellation_token,
            };

            if let Err(e) = Box::pin(current_step.execute(&mut context)).await {
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

        self.execute_job_result(
            &mut job_result,
            &directory,
            &mut merged_parameters,
            &CancellationToken::new(),
        )
        .await
    }

    pub async fn stop_job(&self, id: &str) -> Result<(), String> {
        if let Some(token) = self.cancellation_tokens.lock().await.get(id) {
            token.cancel();
            Ok(())
        } else {
            Err(format!("No active job found with id: {}", id))
        }
    }
}
