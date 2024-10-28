use askama::Template;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use crate::{job::{self, Job}, script::models::Script};

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
    // json_schema: &'a str,
}

#[derive(Deserialize, Default)]
pub struct JobFormQuery {
    #[serde(rename = "from-script-id")]
    from_script_id: Option<String>,
    #[serde(rename = "from-job-id")]
    from_job_id: Option<String>,
}

pub async fn template_jobs() -> Response {
    let jobs = job::Job::get_all();
    if let Err(_) = jobs {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let jobs = jobs.unwrap();
    let template = JobsTemplate { title: "Jobs", jobs };
    Html(template.render().unwrap()).into_response()
}

pub async fn template_update_job(Path(id): Path<String>) -> Response {
    template_job(Some(axum::extract::Path(id)), "Jobs", Default::default()).await
}

pub async fn template_create_job(params: Query<JobFormQuery>) -> Response {
    template_job(None, "Create Job", params).await
}

pub async fn template_job(id: Option<Path<String>>, title: &str, params: Query<JobFormQuery>) -> Response {
    let job = if let Some(id) = id {
        job::Job::get(id.as_str())
    } else {
        Ok(None)
    };
    if let Err(_) = job {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let job = job.unwrap();

    let mut job_yaml = None;
    if let Some(job) = job.as_ref() {
        job_yaml = Some(serde_yaml::to_string(job).unwrap());
    }

    if let Some(from_script_id) = &params.from_script_id {
        let script = Script::get(from_script_id.as_str());
        if let Err(_) = script {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        let script = script.unwrap();
        if let Some(script) = script {
            job_yaml = Some(serde_yaml::to_string(&Job::from(&script)).unwrap());
        }
    }

    if let Some(from_job_id) = &params.from_job_id {
        let job = job::Job::get(from_job_id.as_str());
        if let Err(_) = job {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        let job = job.unwrap();
        if let Some(job) = job {
            job_yaml = Some(serde_yaml::to_string(&job).unwrap());
        }
    }

    // let json_schema = job::Job::get_json_schema();
    // let json_schema_str = serde_json::to_string(&json_schema).unwrap();

    let template = JobTemplate {
        title,
        job: job_yaml.as_deref(),
        // json_schema: &json_schema_str,
    };
    Html(template.render().unwrap()).into_response()
}
