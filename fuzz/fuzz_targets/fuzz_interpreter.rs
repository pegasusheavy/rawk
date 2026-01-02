#![no_main]

use libfuzzer_sys::fuzz_target;
use awk_rs::{Interpreter, Lexer, Parser};
use std::io::{BufReader, Cursor};

fuzz_target!(|data: &[u8]| {
    // Split the data into program and input
    // First 1/3 is the program, rest is input
    let split_point = data.len() / 3;
    let (program_bytes, input_bytes) = data.split_at(split_point);

    let program = match std::str::from_utf8(program_bytes) {
        Ok(s) => s,
        Err(_) => return,
    };

    let input = match std::str::from_utf8(input_bytes) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Limit input sizes to prevent hangs
    if program.len() > 10000 || input.len() > 100000 {
        return;
    }

    // Try to lex
    let mut lexer = Lexer::new(program);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(_) => return,
    };

    // Try to parse
    let mut parser = Parser::new(tokens);
    let ast = match parser.parse() {
        Ok(a) => a,
        Err(_) => return,
    };

    // Try to run with a timeout simulation (limit iterations)
    let mut interpreter = Interpreter::new(&ast);
    let mut output = Vec::new();

    if input.is_empty() {
        let inputs: Vec<BufReader<Cursor<&str>>> = vec![];
        let _ = interpreter.run(inputs, &mut output);
    } else {
        let inputs = vec![BufReader::new(Cursor::new(input))];
        let _ = interpreter.run(inputs, &mut output);
    }
});
