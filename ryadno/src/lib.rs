use std::sync::Arc;

use axum::{
    Extension, body::Body, http::{Request, StatusCode}, response::IntoResponse
};
use minijinja::Environment;

use crate::structs::register_page::{BaseRegistrationPage, RegistrationPage};

pub mod structs;

pub struct PanelBuilder {
    pub registration_page: Option<Arc<dyn RegistrationPage>>,
}

impl PanelBuilder {
    pub fn new() -> Self {
        Self {
            registration_page: None,
        }
    }

    pub fn with_registration<T: RegistrationPage + 'static>(mut self, registration_page: T) -> Self
    {
        self.registration_page = Some(Arc::new(registration_page));
        self
    }

    pub fn build(self, env: Environment<'static>) -> axum::Router {
        let mut router = axum::Router::new();
        let mut state = PanelState {
            mjenv: env,
            register_page_config: None
        };

        if let Some(trigger) = self.registration_page {
            // We create the actual Axum route here
            router = router.route(
                "/register",
                axum::routing::post(async move |req: Request<Body>| {
                    let trigger = trigger.clone();
                    let result = async move { trigger.call(req).await }.await;

                    StatusCode::OK
                }),
            );
        }

        router.layer(Extension(state))
    }
}

#[derive(Clone)]
pub struct PanelState {
    mjenv: Environment<'static>,
    register_page_config: Option<RegisterPageConfig>,
}

#[derive(Clone)]
pub struct RegisterPageConfig {
	template: String,
	handler: Arc<dyn RegistrationPage>
}
