use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct RegisterPage {
    pub template: String,
    pub handler: Arc<Box<
        dyn Fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>>>> + Sync + Send,
    >>,
    pub path: Option<String>,
}

impl RegisterPage {
    pub fn new<F, Fut>(template: String, handler: F, path: Option<String>) -> Self
    where
        F: Fn(String) -> Fut + Sync + Send + 'static,
        Fut: Future<Output = Result<(), String>> + 'static,
    {
        Self {
            template: template,
            handler: Arc::new(Box::new(move |input| Box::pin(handler(input)))),
            path: path,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterPageForm {
    pub login: String,
    pub password: String,
    pub password_conf: String,
}
