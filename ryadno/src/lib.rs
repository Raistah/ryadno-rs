pub mod structs;

use axum::{
    Extension, Form, Router,
    extract::Request,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use minijinja::{Environment, context};
use std::{convert::Infallible, path::PathBuf};
use tower::{Layer, Service};

use crate::structs::register_page::{RegisterPage, RegisterPageForm};

#[deny(missing_docs)]
//* About Ryadno
//*
//* Ryadno is a set of builders that allows developer create admin panel for axum framework

/// This builder allows init panel that is glue for all other features of this crate
pub struct PanelBuilder {
    app_url: Option<String>,
    prefix: Option<String>,
    style_file: Option<PathBuf>,
    auth_middleware: Option<Box<dyn FnOnce(Router) -> Router + Send>>,
    register_page: Option<RegisterPage>,
}

impl<'a> PanelBuilder {
    pub fn new() -> Self {
        PanelBuilder {
            app_url: None,
            prefix: None,
            style_file: None,
            auth_middleware: None,
            register_page: None,
        }
    }

    /// This function produces router that you can merge or nest to your main one or use its on its onw.
    /// To work properly as nested or merged it should understand what the actual "prefix" of router.
    /// example:
    pub fn build(self, env: Environment<'static>) -> Router {
        let prefix: String = self.prefix.unwrap_or("admin".to_string());
        let mut base_url = self.app_url.unwrap_or("".to_string());
        if !base_url.ends_with('/') {
            base_url.push('/');
            base_url.push_str(&prefix);
        }
        let mut state = PanelState {
            base_url: base_url,
            mjenv: env,
            register_page_config: RegisterPageConfig {
                template: "".to_string(),
                path: "".to_string(),
            },
        };
        let mut router =
            Router::new().route(&format!("/{}", &prefix), get(|| async { "Hello admin" }));

        if let Some(auth_middleware) = self.auth_middleware {
            router = auth_middleware(router);
        }

        if let Some(register_page) = self.register_page {
            state.register_page_config.template = register_page.template;
            state.register_page_config.path = register_page.path.unwrap_or("register".to_string());
            let reg_handler = register_page.handler.clone();
            router = router.route(
                &format!("/{}/{}", &prefix, &state.register_page_config.path),
                get(Self::register_page_get_handler).post(async move || {
                    match reg_handler("hehe".to_string()).await {
                        Ok(_) => "hehe".to_string(),
                        Err(_) => "not hehe".to_string(),
                    }
                }),
            );
        }

        router.layer(Extension(state))
    }

    pub fn set_app_url(mut self, app_url: String) -> Self {
        self.app_url = Some(app_url);
        self
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
        let temp = match state
            .mjenv
            .get_template(&state.register_page_config.template)
        {
            Ok(v) => v,
            Err(err) => {
                eprintln!("Cannot find the template: {}", err);
                return (StatusCode::from_u16(404).unwrap()).into_response();
            }
        };

        match temp.render(context!(
            action => format!("{}/{}", state.base_url, state.register_page_config.path)
        )) {
            Ok(html) => (StatusCode::OK, Html(html)).into_response(),
            Err(err) => {
                eprintln!("Error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR).into_response()
            }
        }
    }
}

#[derive(Clone)]
pub struct PanelState {
    base_url: String,
    mjenv: Environment<'static>,
    register_page_config: RegisterPageConfig,
}

#[derive(Clone)]
pub struct RegisterPageConfig {
    template: String,
    path: String,
}
