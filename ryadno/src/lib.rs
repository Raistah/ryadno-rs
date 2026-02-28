use std::sync::Arc;

use axum::{
    Extension,
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
};
use minijinja::Environment;

use crate::structs::register_page::{BaseRegistrationPage, RegistrationPage};

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
        let mut state = PanelState {
            mjenv: env,
            register_page_config: None,
        };

        let template = state.mjenv.get_template("hehe");

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
        router = router.route(
            &format!("/{}/{}", self.prefix, registration_page.get_path()),
            axum::routing::post(async move |req: Request<Body>| {
                let registration_page = registration_page.clone();
                let result = async move { registration_page.handle_registration(req).await }.await;

                StatusCode::OK
            }),
        );

        return router;
    }
}

#[derive(Clone)]
pub struct PanelState {
    mjenv: Arc<Environment<'static>>,
    register_page_config: Option<RegisterPageConfig>,
}

#[derive(Clone)]
pub struct RegisterPageConfig {
    template: String,
    handler: Arc<dyn RegistrationPage>,
}
