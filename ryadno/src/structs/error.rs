use rkyv::rancor;
use thiserror::Error;
use validator::ValidationErrors;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Validation error occured: {0}")]
    ValidationErrors(#[from] ValidationErrors),

    #[error("Template render error: {0}")]
    RenderError(#[from] minijinja::Error),

    #[error("{0}")]
    Message(String),

    #[error("Archive error: {0}")]
    Rkyv(rancor::Error),
}
