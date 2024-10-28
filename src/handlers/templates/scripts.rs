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
    // json_schema: &'a str,
}

pub async fn template_scripts() -> Response {
    let scripts = Script::get_all();
    if let Err(_) = scripts {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let scripts = scripts.unwrap();
    let template = ScriptsTemplate {
        title: "Scripts",
        scripts,
    };
    Html(template.render().unwrap()).into_response()
}

pub async fn template_update_script(Path(id): Path<String>) -> Response {
    template_script(Some(axum::extract::Path(id)), "Scripts").await
}

pub async fn template_create_script() -> Response {
    template_script(None, "Create Script").await
}

pub async fn template_script(id: Option<Path<String>>, title: &str) -> Response {
    let script = if let Some(id) = id {
        Script::get(id.as_str())
    } else {
        Ok(None)
    };
    if let Err(_) = script {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let script = script.unwrap();

    let mut script_yaml = None;
    if let Some(script) = script.as_ref() {
        script_yaml = Some(serde_yaml::to_string(script).unwrap());
    }

    // let json_schema = Script::get_json_schema();
    // let json_schema_str = serde_json::to_string(&json_schema).unwrap();

    let template = ScriptTemplate {
        title,
        script: script_yaml.as_deref(),
        // json_schema: &json_schema_str,
    };
    Html(template.render().unwrap()).into_response()
}
