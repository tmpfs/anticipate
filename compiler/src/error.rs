use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Rexpect(#[from] rexpect::error::Error),
}
