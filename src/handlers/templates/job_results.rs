use askama::Template;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{job::JobResult, log::LogLevel};

#[derive(Template)]
#[template(path = "job-results.html")]
pub struct JobResultsTemplate<'a> {
    title: String,
    has_in_progress: bool,
    job_id_filter: Option<&'a str>,
}

#[derive(Template)]
#[template(path = "job-results-table.html")]
pub struct JobResultsTableTemplate {
    results: Vec<JobResult>,
}

#[derive(Template)]
#[template(path = "job-result.html")]
pub struct JobResultTemplate<'a> {
    title: &'a str,
    result: &'a JobResult,
}

#[derive(Template)]
#[template(path = "job-result-logs.html")]
pub struct JobResultLogsTemplate<'a> {
    logs: Vec<FormattedLog<'a>>,
}

pub struct FormattedLog<'a> {
    pub timestamp: &'a DateTime<Utc>,
    pub level: &'a LogLevel,
    pub message: &'a str,
}

#[derive(Deserialize)]
pub struct JobResultsQuery {
    #[serde(rename = "job-id")]
    job_id: Option<String>,
}

#[derive(Template)]
#[template(path = "job-result-header.html")]
pub struct JobResultHeaderTemplate<'a> {
    result: &'a JobResult,
    now: DateTime<Utc>,
}

#[derive(Template)]
#[template(path = "job-result-steps.html")]
pub struct JobResultStepsTemplate<'a> {
    result: &'a JobResult,
}

pub async fn template_job_results(query: Query<JobResultsQuery>) -> Response {
    match JobResult::get_all(query.job_id.clone()) {
        Ok(results) => {
            let has_in_progress = results.iter().any(|r| r.finished_at.is_none());
            let template = JobResultsTemplate {
                title: "Job Results".to_string(),
                has_in_progress,
                job_id_filter: query.job_id.as_deref(),
            };
            Html(template.render().unwrap()).into_response()
        }
        Err(e) => {
            eprintln!("Failed to get all job results: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn template_job_results_table(query: Query<JobResultsQuery>) -> Response {
    match JobResult::get_all(query.job_id.clone()) {
        Ok(results) => {
            let template = JobResultsTableTemplate { results };
            Html(template.render().unwrap()).into_response()
        }
        Err(e) => {
            eprintln!("Failed to get all job results for table: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn template_job_result(Path(id): Path<String>) -> Response {
    match JobResult::get(&id) {
        Ok(Some(result)) => {
            let template = JobResultTemplate {
                title: "Job Result",
                result: &result,
            };
            Html(template.render().unwrap()).into_response()
        }
        Ok(None) => {
            eprintln!("Job result not found: {}", id);
            StatusCode::NOT_FOUND.into_response()
        }
        Err(e) => {
            eprintln!("Failed to get job result {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn template_job_result_logs(Path(result_id): Path<String>) -> Response {
    match JobResult::get(&result_id) {
        Ok(Some(result)) => {
            if let Ok(logger) = result.logger.lock() {
                match logger.get_logs() {
                    Ok(logs) => {
                        let formatted_logs: Vec<FormattedLog> = logs
                            .iter()
                            .map(|log| FormattedLog {
                                timestamp: &log.timestamp,
                                level: &log.level,
                                message: &log.message,
                            })
                            .collect();

                        let template = JobResultLogsTemplate { logs: formatted_logs };
                        Html(template.render().unwrap()).into_response()
                    }
                    Err(e) => {
                        eprintln!("Failed to get logs for job result {}: {}", result_id, e);
                        StatusCode::INTERNAL_SERVER_ERROR.into_response()
                    }
                }
            } else {
                eprintln!("Failed to lock logger for job result {}", result_id);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
        Ok(None) => {
            eprintln!("Job result not found: {}", result_id);
            StatusCode::NOT_FOUND.into_response()
        }
        Err(e) => {
            eprintln!("Failed to get job result {}: {}", result_id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn template_job_result_header(Path(id): Path<String>) -> Response {
    match JobResult::get(&id) {
        Ok(Some(result)) => {
            let now = Utc::now();
            let template = JobResultHeaderTemplate { result: &result, now };
            Html(template.render().unwrap()).into_response()
        }
        Ok(None) => {
            eprintln!("Job result not found: {}", id);
            StatusCode::NOT_FOUND.into_response()
        }
        Err(e) => {
            eprintln!("Failed to get job result {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn template_job_result_steps(Path(id): Path<String>) -> Response {
    match JobResult::get(&id) {
        Ok(Some(result)) => {
            let template = JobResultStepsTemplate { result: &result };
            Html(template.render().unwrap()).into_response()
        }
        Ok(None) => {
            eprintln!("Job result not found: {}", id);
            StatusCode::NOT_FOUND.into_response()
        }
        Err(e) => {
            eprintln!("Failed to get job result {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
