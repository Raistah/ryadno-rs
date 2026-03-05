use std::sync::Arc;

use axum::{
    Extension,
    body::Body,
    http::{Request, StatusCode},
    response::Html,
    routing::get,
};
use minijinja::{Environment, context};

use crate::structs::register_page::RegistrationPage;

pub mod structs;

pub struct PanelBuilder {
    pub prefix: String,
    pub registration_page: Option<Arc<dyn RegistrationPage>>,
}

impl PanelBuilder {
    pub fn new() -> Self {
        Self {
            prefix: "admin".to_string(),
            registration_page: None,
        }
    }

    pub fn build(self, env: Arc<Environment<'static>>) -> axum::Router {
        let mut router = axum::Router::new();
        let mut state = PanelState { mjenv: env };

        if let Some(registration_page) = &self.registration_page {
            router = self.add_registration(router, registration_page.clone());
        }

        router.layer(Extension(state))
    }

    pub fn with_registration<T: RegistrationPage + 'static>(
        mut self,
        registration_page: T,
    ) -> Self {
        self.registration_page = Some(Arc::new(registration_page));
        self
    }

    fn add_registration(
        &self,
        mut router: axum::Router,
        registration_page: Arc<dyn RegistrationPage>,
    ) -> axum::Router {
        let registration_page_clone = registration_page.clone();
        router = router.route(
            &format!("/{}/{}", self.prefix, registration_page.get_path()),
            get(async move |Extension(panel_state): Extension<PanelState>| {
                match registration_page_clone.render_page(panel_state.mjenv, context! {}) {
                    Ok(v) => (StatusCode::OK, Html(v)),
                    Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Html("".to_string())),
                }
            })
            .post(async move |req: Request<Body>| {
                let registration_page = registration_page.clone();
                match async move { registration_page.handle_registration(req).await }.await {
                    Ok(()) => (StatusCode::OK, "".to_string()),
                    Err(err) => (StatusCode::BAD_REQUEST, err.to_string()),
                }
            }),
        );

        return router;
    }
}

#[derive(Clone)]
pub struct PanelState {
    mjenv: Arc<Environment<'static>>,
}
