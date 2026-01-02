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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location() {
        let loc = SourceLocation::new(10, 5);
        assert_eq!(loc.line, 10);
        assert_eq!(loc.column, 5);
        assert_eq!(format!("{}", loc), "line 10, column 5");
    }

    #[test]
    fn test_lexer_error() {
        let err = Error::lexer("unexpected character", 1, 5);
        assert!(matches!(err, Error::Lexer { .. }));
        let msg = format!("{}", err);
        assert!(msg.contains("lexer error"));
        assert!(msg.contains("unexpected character"));
    }

    #[test]
    fn test_parser_error() {
        let err = Error::parser("expected expression", 2, 10);
        assert!(matches!(err, Error::Parser { .. }));
        let msg = format!("{}", err);
        assert!(msg.contains("parser error"));
    }

    #[test]
    fn test_runtime_error() {
        let err = Error::runtime("division by zero");
        assert!(matches!(err, Error::Runtime { .. }));
        let msg = format!("{}", err);
        assert!(msg.contains("runtime error"));
        assert!(msg.contains("division by zero"));
    }

    #[test]
    fn test_runtime_error_with_location() {
        let err = Error::runtime_at("undefined variable", 5, 3);
        assert!(matches!(err, Error::RuntimeWithLocation { .. }));
        let msg = format!("{}", err);
        assert!(msg.contains("runtime error"));
        assert!(msg.contains("line 5"));
    }

    #[test]
    fn test_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
        let msg = format!("{}", err);
        assert!(msg.contains("I/O error"));
    }

    #[test]
    fn test_regex_error() {
        let re_err = regex::Regex::new("[invalid").unwrap_err();
        let err: Error = re_err.into();
        assert!(matches!(err, Error::Regex(_)));
        let msg = format!("{}", err);
        assert!(msg.contains("regex error"));
    }
}
