use std::{collections::HashMap, error::Error, sync::Arc};

use axum::{
    Extension, Json, Router, extract,
    http::StatusCode,
    response::Html,
    routing::{get, post},
};
use minijinja::{Environment, context, path_loader};
use redis::{Client, TypedCommands};
use ryadno::{
    fields::{Field, text_input::TextField},
    form::{FieldTypes, Form, FormContext, UpdatePayload, ValueWrapper},
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
	let client = redis::Client::open("redis://127.0.0.1/")?;
	let mut con = client.get_connection()?;
	let form = Form {
		schema: vec![FieldTypes::Text(TextField::make("test".to_string()))],
		uuid: "hehe".to_string(),
		update_endpoint: "hehe".to_string(),
		data: Some(ValueWrapper(json!("hehe")))
	};

	println!("before: {:?}", &form);
	let serialized = form.serialize().unwrap();
	println!("serialized: {:?}", &serialized);
	let resp = con.set("test", &serialized);
	println!("set: {:?}", &resp);
	let resp: Option<Vec<u8>> = redis::cmd("GET").arg("test").query(&mut con)?;
	println!("get: {:?}", &resp);
	let form = Form::from_bytes(resp.unwrap());
	println!("deserialized: {:?}", form);

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
                    extra: HashMap::new(),
                };
                let html = field
                    .to_html(mjenv.as_ref(), "test".to_string(), None, &form_context)
                    .unwrap();

                let page_templ = mjenv.get_template("base.jinja").unwrap();
                let page = page_templ
                    .render(context! {
                        html => html
                    })
                    .unwrap();

                (StatusCode::OK, Html(page))
            }),
        )
        .route(
            "/form/update/",
            post(
                async |extract::Json(payload): extract::Json<UpdatePayload>| {
                    println!("{:?} {}", payload, payload.value.as_str().unwrap());
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            payload.uuid: {
                                "value": format!("{}|", payload.value.as_str().unwrap_or(""))
                            }
                        })),
                    )
                },
            ),
        )
        .layer(Extension(env));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
    Ok(())
}
