use std::sync::Arc;

use axum::{body::Body, extract::FromRequest, http::Request};
use futures::future::BoxFuture;
use minijinja::Environment;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::structs::error::Error;

pub struct BaseRegistrationPage<T, F> {
    pub handler: F,
    pub template: String,
    pub path: Option<String>,
    _marker: std::marker::PhantomData<fn(T)>,
}

pub trait RegistrationPage: Send + Sync {
    fn handle_registration(&self, req: Request<Body>) -> BoxFuture<'static, Result<(), Error>>;
    fn render_page(
        &self,
        env: Arc<Environment<'static>>,
        ctx: minijinja::Value,
    ) -> Result<String, minijinja::Error>;
    fn get_path(&self) -> String;
}

impl<T, F, Fut> BaseRegistrationPage<T, F>
where
    T: serde::de::DeserializeOwned + Send + 'static,
    F: Fn(T) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + Send,
{
    pub fn new(handler: F, template: String, path: Option<String>) -> Self {
        Self {
            handler,
            template,
            path,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T, F, Fut> RegistrationPage for BaseRegistrationPage<T, F>
where
    T: serde::de::DeserializeOwned + Validate + Send + 'static,
    F: Fn(T) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + Send,
{
    fn handle_registration(&self, req: Request<Body>) -> BoxFuture<'static, Result<(), Error>> {
        let handler = self.handler.clone();
        Box::pin(async move {
            match axum::extract::Form::<T>::from_request(req, &()).await {
                Ok(axum::extract::Form(form_data)) => {
                    match form_data.validate() {
                        Ok(()) => (),
                        Err(err) => {
                            return Err(Error::ValidationErrors(err));
                        }
                    };

                    match handler(form_data).await {
                        Ok(()) => Ok(()),
                        Err(v) => Err(Error::Message(v)),
                    }
                }
                Err(_) => Err(Error::Message("Deserialization Error".to_string())),
            }
        })
    }

    fn render_page(
        &self,
        env: Arc<Environment<'static>>,
        ctx: minijinja::Value,
    ) -> Result<String, minijinja::Error> {
        let template = env.get_template(&self.template)?;
        template.render(ctx)
    }

    fn get_path(&self) -> String {
        self.path.clone().unwrap_or("register".to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct RegisterPageForm {
    #[validate(length(min = 1, max = 20))]
    pub email: String,
    #[validate(length(min = 5, max = 30))]
    pub password: String,
    #[validate(must_match(other = "password"))]
    pub password_conf: String,
    pub register_token: String,
}
