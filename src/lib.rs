//! rawk - A 100% POSIX-compatible AWK implementation in Rust
//!
//! This crate provides a complete AWK interpreter that aims for full compatibility
//! with POSIX AWK and GNU AWK extensions.

pub mod ast;
pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod value;

pub use error::{Error, Result, SourceLocation};
pub use interpreter::Interpreter;
pub use lexer::{Lexer, Token, TokenKind};
pub use parser::Parser;
pub use value::Value;
