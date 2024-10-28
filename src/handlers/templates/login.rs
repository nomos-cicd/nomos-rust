use askama::Template;
use axum::{
    extract::Query,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;

use crate::handlers::auth::{self, Credentials};

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    next: Option<String>,
    title: String,
}

#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

pub async fn template_get_login(Query(NextUrl { next }): Query<NextUrl>) -> Html<String> {
    let template = LoginTemplate {
        next,
        title: "Login".to_string(),
    };
    template.render().unwrap().into()
}

pub async fn template_post_login(
    mut auth_session: auth::AuthSession,
    Form(creds): Form<Credentials>,
) -> impl IntoResponse {
    let user = match auth_session.authenticate(creds.clone()).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            let mut login_url = "/login".to_string();
            if let Some(next) = creds.next {
                login_url = format!("{}?next={}", login_url, next);
            };

            return Redirect::to(&login_url).into_response();
        }
        Err(_) => return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if auth_session.login(&user).await.is_err() {
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if let Some(ref next) = creds.next {
        Redirect::to(next)
    } else {
        Redirect::to("/")
    }
    .into_response()
}
