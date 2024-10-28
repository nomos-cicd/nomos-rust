use axum::{
    extract::Path,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use crate::script::models::Script;

pub async fn get_scripts() -> Response {
    match Script::get_all() {
        Ok(scripts) => Json(scripts).into_response(),
        Err(e) => {
            eprintln!("Failed to get scripts: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_script(Path(id): Path<String>) -> Response {
    match Script::get(id.as_str()) {
        Ok(Some(script)) => Json(script).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            eprintln!("Failed to get script {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_script(headers: HeaderMap, body: String) -> Response {
    let content_type = match headers.get("content-type") {
        Some(ct) => ct.to_str().unwrap_or(""),
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    if content_type != "application/yaml" {
        return StatusCode::BAD_REQUEST.into_response();
    }

    match serde_yaml::from_str::<Script>(&body) {
        Ok(script) => match script.sync(None) {
            Ok(_) => Json(script).into_response(),
            Err(e) => {
                eprintln!("Failed to sync script: {}", e);
                StatusCode::BAD_REQUEST.into_response()
            }
        },
        Err(e) => {
            eprintln!("Failed to parse script YAML: {}", e);
            StatusCode::BAD_REQUEST.into_response()
        }
    }
}

pub async fn delete_script(Path(id): Path<String>) -> Response {
    match Script::get(id.as_str()) {
        Ok(Some(script)) => match script.delete() {
            Ok(_) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => {
                eprintln!("Failed to delete script {}: {}", id, e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            eprintln!("Failed to get script for deletion {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
