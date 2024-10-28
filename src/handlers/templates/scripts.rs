use askama::Template;
use axum::{
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

use crate::script::models::Script;

#[derive(Template)]
#[template(path = "scripts.html")]
pub struct ScriptsTemplate<'a> {
    title: &'a str,
    scripts: Vec<Script>,
}

#[derive(Template)]
#[template(path = "script.html")]
pub struct ScriptTemplate<'a> {
    title: &'a str,
    script: Option<&'a str>,
}

pub async fn template_scripts() -> Response {
    match Script::get_all() {
        Ok(scripts) => {
            let template = ScriptsTemplate {
                title: "Scripts",
                scripts,
            };
            Html(template.render().unwrap()).into_response()
        }
        Err(e) => {
            eprintln!("Failed to get all scripts: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn template_update_script(Path(id): Path<String>) -> Response {
    template_script(Some(axum::extract::Path(id)), "Scripts").await
}

pub async fn template_create_script() -> Response {
    template_script(None, "Create Script").await
}

pub async fn template_script(id: Option<Path<String>>, title: &str) -> Response {
    let script = if let Some(id) = id {
        match Script::get(id.as_str()) {
            Ok(script) => script,
            Err(e) => {
                eprintln!("Failed to get script {}: {}", id.as_str(), e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    } else {
        None
    };

    let mut script_yaml = None;
    if let Some(script) = script.as_ref() {
        script_yaml = Some(serde_yaml::to_string(script).unwrap());
    }

    let template = ScriptTemplate {
        title,
        script: script_yaml.as_deref(),
    };
    Html(template.render().unwrap()).into_response()
}
