use axum::{
    body::Body,
    extract::FromRequest,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

pub struct BaseRegistrationPage<T, F> {
    pub handler: F,
    pub template: String,
    _marker: std::marker::PhantomData<fn(T)>,
}

// This is what PanelBuilder stores.
// It erases the generic 'T' (the Form) so the Builder stays simple.
pub trait RegistrationPage: Send + Sync {
    fn call(&self, req: Request<Body>) -> BoxFuture<'static, Response>;
}

impl<T, F, Fut> BaseRegistrationPage<T, F>
where
    T: serde::de::DeserializeOwned + Send + 'static,
    F: Fn(T) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + Send,
{
    pub fn new(handler: F, template: String, ) -> Self {
        Self {
            handler,
            template,
            _marker: std::marker::PhantomData
        }
    }
}

impl<T, F, Fut> RegistrationPage for BaseRegistrationPage<T, F>
where
    T: serde::de::DeserializeOwned + Send + 'static,
    F: Fn(T) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + Send,
{
    fn call(&self, req: Request<Body>) -> BoxFuture<'static, Response> {
        let handler = self.handler.clone();
        Box::pin(async move {
            // WE handle the Axum extraction here so the user doesn't have to
            match axum::extract::Form::<T>::from_request(req, &()).await {
                Ok(axum::extract::Form(form_data)) => match handler(form_data).await {
                    Ok(_) => (StatusCode::OK, "Success").into_response(),
                    Err(v) => (StatusCode::UNAUTHORIZED, v).into_response(),
                },
                Err(_) => (StatusCode::BAD_REQUEST, "Deserialization Error").into_response(),
            }
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterPageForm {
    pub login: String,
    pub password: String,
    pub password_conf: String,
    pub register_token: String,
}
