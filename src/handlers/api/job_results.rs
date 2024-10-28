use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::job::JobResult;

#[derive(Deserialize)]
pub struct JobResultsQuery {
    #[serde(rename = "job-id")]
    job_id: Option<String>,
}

pub async fn get_job_results(query: Query<JobResultsQuery>) -> Response {
    match JobResult::get_all(query.job_id.clone()) {
        Ok(results) => Json(results).into_response(),
        Err(e) => {
            eprintln!("Failed to get job results: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_job_result(Path(id): Path<String>) -> Response {
    match JobResult::get(id.as_str()) {
        Ok(Some(result)) => Json(result).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(JobResult::create_dummy())).into_response(),
        Err(e) => {
            eprintln!("Failed to get job result {}: {}", id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(JobResult::create_dummy())).into_response()
        }
    }
}
