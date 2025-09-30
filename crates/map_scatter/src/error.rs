//! Error types and result alias for the crate.
//!
//! This module defines [`enum@crate::error::Error`] and the crate-wide [Result] alias. Variants cover
//! invalid configuration, field graph compile/runtime failures, missing resources,
//! IO, and generic errors.
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("fieldgraph compile error: {0}")]
    Compile(String),

    #[error("field runtime error: {0}")]
    Runtime(String),

    #[error("missing texture '{id}'")]
    MissingTexture { id: String },

    #[error("unknown field '{id}'")]
    UnknownField { id: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error::Other(value)
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Error::Other(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_string_uses_other_variant() {
        let err: Error = String::from("boom").into();
        matches!(err, Error::Other(_))
            .then_some(())
            .expect("expected Other variant");
    }

    #[test]
    fn from_str_allocates_owned_message() {
        let err: Error = "issue".into();
        assert!(matches!(err, Error::Other(ref msg) if msg == "issue"));
    }
}
