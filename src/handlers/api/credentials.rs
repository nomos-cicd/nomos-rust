use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::credential::Credential;

pub async fn get_credentials() -> Response {
    match Credential::get_all() {
        Ok(credentials) => Json(credentials).into_response(),
        Err(e) => {
            eprintln!("Failed to get credentials: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_credential(Path(id): Path<String>) -> Response {
    match Credential::get(id.as_str(), None) {
        Ok(Some(credential)) => Json(credential).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            eprintln!("Failed to get credential {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_credential(Json(credential): Json<Credential>) -> Response {
    match credential.sync(&mut None) {
        Ok(_) => Json(credential).into_response(),
        Err(e) => {
            eprintln!("Failed to sync credential: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_credential(Path(id): Path<String>) -> Response {
    match Credential::get(id.as_str(), None) {
        Ok(Some(credential)) => match credential.delete() {
            Ok(_) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => {
                eprintln!("Failed to delete credential {}: {}", id, e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            eprintln!("Failed to get credential for deletion {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
