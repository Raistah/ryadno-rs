use rkyv::rancor;
use thiserror::Error;
use validator::ValidationErrors;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Validation error occured")]
    ValidationErrors(#[from] ValidationErrors),

    #[error("0")]
    Message(String),

    #[error("Archive error: {0}")]
    Rkyv(rancor::Error),
}
