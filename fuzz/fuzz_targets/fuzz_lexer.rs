#![no_main]

use libfuzzer_sys::fuzz_target;
use awk_rs::Lexer;

fuzz_target!(|data: &str| {
    // Fuzz the lexer with arbitrary input strings
    let mut lexer = Lexer::new(data);
    // We don't care if it fails, just that it doesn't panic or hang
    let _ = lexer.tokenize();
});
