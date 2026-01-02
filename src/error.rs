use std::fmt;
use thiserror::Error;

/// Location in source code for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

impl SourceLocation {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// All error types for rawk
#[derive(Error, Debug)]
pub enum Error {
    #[error("lexer error at {location}: {message}")]
    Lexer {
        message: String,
        location: SourceLocation,
    },

    #[error("parser error at {location}: {message}")]
    Parser {
        message: String,
        location: SourceLocation,
    },

    #[error("runtime error: {message}")]
    Runtime { message: String },

    #[error("runtime error at {location}: {message}")]
    RuntimeWithLocation {
        message: String,
        location: SourceLocation,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("regex error: {0}")]
    Regex(#[from] regex::Error),
}

impl Error {
    pub fn lexer(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self::Lexer {
            message: message.into(),
            location: SourceLocation::new(line, column),
        }
    }

    pub fn parser(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self::Parser {
            message: message.into(),
            location: SourceLocation::new(line, column),
        }
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime {
            message: message.into(),
        }
    }

    pub fn runtime_at(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self::RuntimeWithLocation {
            message: message.into(),
            location: SourceLocation::new(line, column),
        }
    }
}

/// Result type alias for rawk operations
pub type Result<T> = std::result::Result<T, Error>;
