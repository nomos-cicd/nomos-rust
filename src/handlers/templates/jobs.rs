use askama::Template;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use crate::{
    job::{self, Job},
    script::models::Script,
};

#[derive(Template)]
#[template(path = "jobs.html")]
pub struct JobsTemplate<'a> {
    title: &'a str,
    jobs: Vec<job::Job>,
}

#[derive(Template)]
#[template(path = "job.html")]
pub struct JobTemplate<'a> {
    title: &'a str,
    job: Option<&'a str>,
}

#[derive(Deserialize, Default)]
pub struct JobFormQuery {
    #[serde(rename = "from-script-id")]
    from_script_id: Option<String>,
    #[serde(rename = "from-job-id")]
    from_job_id: Option<String>,
}

pub async fn template_jobs() -> Response {
    match job::Job::get_all() {
        Ok(jobs) => {
            let template = JobsTemplate { title: "Jobs", jobs };
            Html(template.render().unwrap()).into_response()
        }
        Err(e) => {
            eprintln!("Failed to get all jobs: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn template_update_job(Path(id): Path<String>) -> Response {
    template_job(Some(axum::extract::Path(id)), "Jobs", Default::default()).await
}

pub async fn template_create_job(params: Query<JobFormQuery>) -> Response {
    template_job(None, "Create Job", params).await
}

pub async fn template_job(id: Option<Path<String>>, title: &str, params: Query<JobFormQuery>) -> Response {
    let job = if let Some(id) = id {
        match job::Job::get(id.as_str()) {
            Ok(job) => job,
            Err(e) => {
                eprintln!("Failed to get job {}: {}", id.as_str(), e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    } else {
        None
    };

    let mut job_yaml = None;
    if let Some(job) = job.as_ref() {
        job_yaml = Some(serde_yaml::to_string(job).unwrap());
    }

    if let Some(from_script_id) = &params.from_script_id {
        match Script::get(from_script_id.as_str()) {
            Ok(script) => {
                if let Some(script) = script {
                    job_yaml = Some(serde_yaml::to_string(&Job::from(&script)).unwrap());
                }
            }
            Err(e) => {
                eprintln!("Failed to get script {}: {}", from_script_id, e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    }

    if let Some(from_job_id) = &params.from_job_id {
        match job::Job::get(from_job_id.as_str()) {
            Ok(job) => {
                if let Some(job) = job {
                    job_yaml = Some(serde_yaml::to_string(&job).unwrap());
                }
            }
            Err(e) => {
                eprintln!("Failed to get job {}: {}", from_job_id, e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    }

    let template = JobTemplate {
        title,
        job: job_yaml.as_deref(),
    };
    Html(template.render().unwrap()).into_response()
}
