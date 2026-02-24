use axum::{body::Body, extract::FromRequest, http::Request};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub struct BaseRegistrationPage<T, F> {
    pub handler: F,
    pub template: String,
    pub path: Option<String>,
    _marker: std::marker::PhantomData<fn(T)>,
}

// This is what PanelBuilder stores.
// It erases the generic 'T' (the Form) so the Builder stays simple.
pub trait RegistrationPage: Send + Sync {
    fn handle_registration(&self, req: Request<Body>) -> BoxFuture<'static, Result<(), String>>;
    fn get_template(&self) -> String;
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

// TODO: i need to choose what i leave to dev, i think it is good to make parsing and validating, and then pass form data to handler.
// in handler dev should actually register user and return result
impl<T, F, Fut> RegistrationPage for BaseRegistrationPage<T, F>
where
    T: serde::de::DeserializeOwned + Send + 'static,
    F: Fn(T) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + Send,
{
    fn handle_registration(&self, req: Request<Body>) -> BoxFuture<'static, Result<(), String>> {
        let handler = self.handler.clone();
        Box::pin(async move {
            // WE handle the Axum extraction here so the user doesn't have to
            match axum::extract::Form::<T>::from_request(req, &()).await {
                Ok(axum::extract::Form(form_data)) => match handler(form_data).await {
                    Ok(_) => Ok(()),
                    Err(v) => Err(v),
                },
                Err(_) => Err("Deserialization Error".to_string()),
            }
        })
    }

        fn get_template(&self) -> String {
            self.template.clone()
        }

    fn get_path(&self) -> String {
        self.path.clone().unwrap_or("register".to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct RegisterPageForm {
    pub login: String,
    pub password: String,
    pub password_conf: String,
    pub register_token: String,
}
