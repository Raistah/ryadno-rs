use std::{collections::HashMap, sync::Arc};

use axum::{Extension, Router, http::StatusCode, response::Html, routing::get};
use minijinja::{Environment, context, path_loader};
use ryadno::{fields::{Field, text_input::TextField}, form::FormContext};

#[tokio::main]
async fn main() {
    let mut env = Environment::new();
    env.set_loader(path_loader("templates"));
    let env = Arc::new(env);
    let router = Router::new()
        .route(
            "/",
            get(async |Extension(mjenv): Extension<Arc<Environment>>| {
                let field = TextField::make("test".to_string());
                let mut headers: HashMap<String, String> = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                let form_context = FormContext {
               		update_endpoint: "/form/update/".to_string(),
                	headers: headers,
                	extra: HashMap::new()
                };
                let html = field.to_html(mjenv.as_ref(), "test".to_string(), None, &form_context).unwrap();

                let page_templ = mjenv.get_template("base.jinja").unwrap();
                let page = page_templ
                    .render(context! {
                        html => html
                    })
                    .unwrap();

                (StatusCode::OK, Html(page))
            }),
        )
        .layer(Extension(env));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
