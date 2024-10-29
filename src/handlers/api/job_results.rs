use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{oneshot, Mutex};

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

pub async fn stop_job(
    Path(id): Path<String>,
    State(abort_senders): State<Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>>,
) -> Response {
    let mut abort_senders = abort_senders.lock().await;
    if let Some(abort_sender) = abort_senders.remove(&id) {
        if abort_sender.send(()).is_ok() {
            StatusCode::OK.into_response()
        } else {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
