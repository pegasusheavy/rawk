#![no_main]

use libfuzzer_sys::fuzz_target;
use awk_rs::{Lexer, Parser};

fuzz_target!(|data: &str| {
    // Try to lex the input
    let mut lexer = Lexer::new(data);
    if let Ok(tokens) = lexer.tokenize() {
        // Try to parse if lexing succeeds
        let mut parser = Parser::new(tokens);
        let _ = parser.parse();
    }
});
