use std::str::FromStr;

use askama::Template;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use crate::credential::{Credential, CredentialType};

#[derive(Template)]
#[template(path = "credentials.html")]
pub struct CredentialsTemplate<'a> {
    title: &'a str,
    credentials: Vec<Credential>,
}

pub async fn template_credentials() -> Response {
    match Credential::get_all() {
        Ok(credentials) => {
            let template = CredentialsTemplate {
                title: "Credentials",
                credentials,
            };
            Html(template.render().unwrap()).into_response()
        }
        Err(e) => {
            eprintln!("Failed to get all credentials: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Template)]
#[template(path = "credential.html")]
pub struct CredentialTemplate<'a> {
    title: &'a str,
    credential: Option<&'a Credential>,
    credential_value: &'a CredentialType,
}

pub async fn template_update_credential(Path(id): Path<String>) -> Response {
    template_credential(Some(axum::extract::Path(id)), "Credentials").await
}

pub async fn template_create_credential() -> Response {
    template_credential(None, "Create Credentials").await
}

pub async fn template_credential(id: Option<Path<String>>, title: &str) -> Response {
    let credential = if let Some(id) = id {
        match Credential::get(id.as_str(), None) {
            Ok(cred) => cred,
            Err(e) => {
                eprintln!("Failed to get credential {}: {}", id.as_str(), e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    } else {
        None
    };

    let default_value = CredentialType::from_str("ssh").unwrap();

    let credential_value = if let Some(cred) = credential.as_ref() {
        &cred.value
    } else {
        &default_value
    };

    let template = CredentialTemplate {
        title,
        credential: credential.as_ref(),
        credential_value,
    };
    Html(template.render().unwrap()).into_response()
}

#[derive(Template)]
#[template(path = "credential-value.html")]
pub struct CredentialValueTemplate<'a> {
    credential_value: &'a CredentialType,
}

#[derive(Deserialize)]
pub struct CredentialValueQuery {
    id: Option<String>,
    #[serde(rename = "type")]
    credential_type: String,
}

pub async fn template_credential_value(params: Query<CredentialValueQuery>) -> Response {
    if let Some(id) = &params.id {
        match Credential::get(id.as_str(), None) {
            Ok(credential) => {
                if let Some(credential) = credential {
                    if credential.get_credential_type() == params.credential_type {
                        let template = CredentialValueTemplate {
                            credential_value: &credential.value,
                        };
                        return (StatusCode::OK, Html(template.render().unwrap())).into_response();
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to get credential {}: {}", id, e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    }

    let credential_type = CredentialType::from_str(params.credential_type.as_str());
    if let Ok(credential_type) = credential_type {
        let template = CredentialValueTemplate {
            credential_value: &credential_type,
        };
        return Html(template.render().unwrap()).into_response();
    }

    StatusCode::BAD_REQUEST.into_response()
}
