use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreError {
    InvalidId(String),
    InvalidState(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(message) => write!(f, "invalid id: {message}"),
            Self::InvalidState(message) => write!(f, "invalid state: {message}"),
        }
    }
}

impl std::error::Error for CoreError {}
