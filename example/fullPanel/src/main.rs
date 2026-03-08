use std::sync::Arc;

use axum::{Extension, Router, http::StatusCode, response::Html, routing::get};
use minijinja::{Environment, context, path_loader};
use ryadno::fields::{Field, text_input::TextField};

#[tokio::main]
async fn main() {
    let mut env = Environment::new();
    env.set_loader(path_loader("templates"));
    let env = Arc::new(env);
    let router = Router::new()
        .route(
            "/",
            get(async |Extension(mjenv): Extension<Arc<Environment>>| {
                let mut field = TextField::make("test".to_string());
                // let context = field.prepare_context(serde_json::Value::Null);
                let context = context! {
                	state_path => "test"
                };
                let html = field.to_html(mjenv.as_ref(), context).unwrap();

                let page_templ = mjenv.get_template("ryadno/base.jinja").unwrap();
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
