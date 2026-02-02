use core::fmt;

#[derive(Debug)]
pub enum Error {
    EOF
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EOF => write!(f, "Reached EOF")
        }
    }
}

impl std::error::Error for Error { }