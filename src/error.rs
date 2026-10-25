//! Crate-wide error handling.
//!
//! Every fallible function returns [`Result`]. The error is a boxed trait
//! object, so `?` works on any standard error type without extra glue.

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

/// Shorthand for returning an error built from a message.
pub fn err<T>(message: impl Into<String>) -> Result<T> {
    let text: String = message.into();
    Err(text.into())
}
