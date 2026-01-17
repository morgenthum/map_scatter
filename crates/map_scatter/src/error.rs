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
    InvalidConfig(
        /// Description of the invalid configuration.
        String,
    ),

    #[error("fieldgraph compile error: {0}")]
    Compile(
        /// Description of the compile error.
        String,
    ),

    #[error("field runtime error: {0}")]
    Runtime(
        /// Description of the runtime error.
        String,
    ),

    #[error("missing texture '{id}'")]
    MissingTexture {
        /// Identifier of the missing texture.
        id: String,
    },

    #[error("unknown field '{id}'")]
    UnknownField {
        /// Identifier of the unknown field.
        id: String,
    },

    #[error(transparent)]
    Io(
        /// Source IO error.
        #[from]
        std::io::Error,
    ),

    #[error("{0}")]
    Other(
        /// Generic error message.
        String,
    ),
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
