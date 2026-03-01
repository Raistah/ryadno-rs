use serde::Serialize;
use thiserror::Error;
use validator::ValidationErrors;

#[derive(Error, Debug, Serialize)]
pub enum Error {
    #[error("Validation error occured")]
    ValidationErrors(#[from] ValidationErrors),

    #[error("0")]
    Message(String),
}
