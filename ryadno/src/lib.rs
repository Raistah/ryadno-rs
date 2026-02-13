pub mod structs;

use axum::{
    Extension, Router,
    extract::Request,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use minijinja::{Environment, context};
use std::{convert::Infallible, path::PathBuf};
use tower::{Layer, Service};

use crate::structs::register_page::RegisterPage;

#[deny(missing_docs)]
//* About Ryadno
//*
//* Ryadno is a set of builders that allows developer create admin panel for axum framework

/// This builder allows init panel that is glue for all other features of this crate
pub struct PanelBuilder {
    prefix: Option<String>,
    style_file: Option<PathBuf>,
    auth_middleware: Option<Box<dyn FnOnce(Router) -> Router + Send>>,
    register_page: Option<RegisterPage>,
}

impl<'a> PanelBuilder {
    pub fn new() -> Self {
        PanelBuilder {
            prefix: None,
            style_file: None,
            auth_middleware: None,
            register_page: None,
        }
    }

    pub fn build(self, env: Environment<'static>) -> Router {
        let prefix: String = self.prefix.unwrap_or("admin".to_string());
        let mut state = PanelState {
            mjenv: env,
            register_page_template: "".to_string(),
        };
        let mut router =
            Router::new().route(&format!("/{}", &prefix), get(|| async { "Hello admin" }));

        if let Some(auth_middleware) = self.auth_middleware {
            router = auth_middleware(router);
        }

        if let Some(register_page) = self.register_page {
            state.register_page_template = register_page.template;
            router = router.route(
                &format!(
                    "/{}/{}",
                    &prefix,
                    &register_page.path.unwrap_or("register".to_string())
                ),
                get(Self::register_page_get_handler),
            );
        }

        router.layer(Extension(state))
    }

    pub fn set_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }

    pub fn set_style_file(mut self, path: PathBuf) -> Self {
        self.style_file = Some(path);
        self
    }

    pub fn set_register_page(mut self, page: RegisterPage) -> Self {
        self.register_page = Some(page);
        self
    }

    pub fn set_auth_middleware<L>(mut self, layer: L) -> Self
    where
        L: Layer<axum::routing::Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request, Response = Response, Error = Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
    {
        self.auth_middleware = Some(Box::new(move |r| r.layer(layer)));
        self
    }

    pub async fn register_page_get_handler(
        Extension(state): Extension<PanelState>,
    ) -> impl IntoResponse {
        println!("hehe");
        let temp = match state.mjenv.get_template(&state.register_page_template) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("Cannot find the template: {}", err);
                return (StatusCode::from_u16(404).unwrap()).into_response();
            }
        };
        println!("hehe 1");

        match temp.render(context!()) {
            Ok(html) => {
           		println!("html: {}", html);
            	(StatusCode::OK, Html(html)).into_response()
            },
            Err(err) => {
                eprintln!("Error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR).into_response()
            }
        }
    }
}

#[derive(Clone)]
pub struct PanelState {
    mjenv: Environment<'static>,
    register_page_template: String,
}
