use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    job::models::{Job, JobResult},
    script::{models::Script, ScriptExecutionContext, ScriptExecutor, ScriptParameterType},
};

pub struct JobExecutor;

impl JobExecutor {
    pub fn execute_with_script(
        job: &Job,
        parameters: HashMap<String, ScriptParameterType>,
        script: &Script,
    ) -> Result<String, String> {
        job.validate_parameters(Some(script))?;

        let mut merged_parameters = job.merged_parameters(Some(script), parameters.clone())?;
        let mut job_result = JobResult::try_from((job, script, false))?;
        let id = job_result.id.clone();

        let directory = crate::job::utils::default_job_results_location()?.join(&job_result.id);
        fs::create_dir_all(&directory).map_err(|e| format!("Failed to create job result directory: {}", e))?;

        job_result.save()?;

        tokio::spawn(async move {
            if let Err(e) = Self::execute_job_result(&mut job_result, &directory, &mut merged_parameters) {
                eprintln!("Job execution failed: {}", e);
            }
        });

        Ok(id)
    }

    pub fn execute_job_result(
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
            if let Err(e) = current_step.execute(&mut context) {
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

    pub fn validate(
        job: &Job,
        script: &Script,
        parameters: HashMap<String, ScriptParameterType>,
    ) -> Result<(), String> {
        let mut merged_parameters = job.merged_parameters(Some(script), parameters)?;
        let mut job_result = JobResult::try_from((job, script, true))?;
        let directory = PathBuf::from("tmp");

        Self::execute_job_result(&mut job_result, &directory, &mut merged_parameters)
    }
}
