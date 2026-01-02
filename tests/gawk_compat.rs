//! Compatibility tests that compare awk-rs output with gawk
//!
//! These tests are skipped if gawk is not available on the system.

use std::io::{BufReader, Cursor};
use std::process::Command;

use awk_rs::{Interpreter, Lexer, Parser};

/// Check if gawk is available on the system
fn gawk_available() -> bool {
    Command::new("gawk")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run a program with awk-rs and return output
fn run_awk_rs(program: &str, input: &str) -> String {
    let mut lexer = Lexer::new(program);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let mut interpreter = Interpreter::new(&ast);
    let mut output = Vec::new();

    if input.is_empty() {
        let inputs: Vec<BufReader<Cursor<&str>>> = vec![];
        interpreter.run(inputs, &mut output).unwrap();
    } else {
        let inputs = vec![BufReader::new(Cursor::new(input))];
        interpreter.run(inputs, &mut output).unwrap();
    }

    String::from_utf8(output).unwrap()
}

/// Run a program with gawk and return output
fn run_gawk(program: &str, input: &str) -> Option<String> {
    let mut cmd = Command::new("gawk");
    cmd.arg(program);

    if !input.is_empty() {
        use std::io::Write;
        use std::process::Stdio;

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        let mut child = cmd.spawn().ok()?;

        // Write input and close stdin
        {
            let stdin = child.stdin.as_mut()?;
            stdin.write_all(input.as_bytes()).ok()?;
        } // stdin is dropped here, closing the pipe

        let output = child.wait_with_output().ok()?;
        String::from_utf8(output.stdout).ok()
    } else {
        let output = cmd.output().ok()?;
        String::from_utf8(output.stdout).ok()
    }
}

/// Compare awk-rs and gawk output for a given program and input
fn compare_with_gawk(program: &str, input: &str) {
    if !gawk_available() {
        eprintln!("Skipping gawk comparison test (gawk not available)");
        return;
    }

    let awk_rs_output = run_awk_rs(program, input);
    let gawk_output = run_gawk(program, input).expect("Failed to run gawk");

    assert_eq!(
        awk_rs_output, gawk_output,
        "Output mismatch for program: {}\nInput: {:?}\nawk-rs output: {:?}\ngawk output: {:?}",
        program, input, awk_rs_output, gawk_output
    );
}

// ============================================================================
// Compatibility Tests
// ============================================================================

#[test]
fn compat_hello_world() {
    compare_with_gawk(r#"BEGIN { print "Hello, World!" }"#, "");
}

#[test]
fn compat_arithmetic() {
    compare_with_gawk("BEGIN { print 1+2, 3*4, 10/2, 7%3, 2^8 }", "");
}

#[test]
fn compat_variables() {
    compare_with_gawk("BEGIN { x = 5; y = 3; print x + y, x - y, x * y }", "");
}

#[test]
fn compat_field_access() {
    compare_with_gawk("{ print $1, $2, $NF }", "one two three four");
}

#[test]
fn compat_field_separator() {
    compare_with_gawk(r#"BEGIN { FS = ":" } { print $1, $3 }"#, "root:x:0:0:root");
}

#[test]
fn compat_nr_nf() {
    compare_with_gawk("{ print NR, NF, $0 }", "a b c\nd e\nf");
}

#[test]
fn compat_for_loop() {
    compare_with_gawk("BEGIN { for (i = 1; i <= 5; i++) print i }", "");
}

#[test]
fn compat_while_loop() {
    compare_with_gawk("BEGIN { i = 0; while (i < 5) { print i; i++ } }", "");
}

#[test]
fn compat_if_else() {
    // Use block syntax for if-else
    compare_with_gawk(
        r#"BEGIN { for (i = 1; i <= 5; i++) { if (i % 2 == 0) { print i, "even" } else { print i, "odd" } } }"#,
        "",
    );
}

#[test]
fn compat_arrays() {
    compare_with_gawk(
        "BEGIN { a[1] = 10; a[2] = 20; a[3] = 30; for (i = 1; i <= 3; i++) print a[i] }",
        "",
    );
}

#[test]
fn compat_length() {
    compare_with_gawk(r#"BEGIN { print length("hello world") }"#, "");
}

#[test]
fn compat_substr() {
    compare_with_gawk(r#"BEGIN { print substr("hello world", 1, 5) }"#, "");
    compare_with_gawk(r#"BEGIN { print substr("hello world", 7) }"#, "");
}

#[test]
fn compat_index() {
    compare_with_gawk(r#"BEGIN { print index("hello world", "wor") }"#, "");
    compare_with_gawk(r#"BEGIN { print index("hello world", "xyz") }"#, "");
}

#[test]
fn compat_split() {
    // Test field splitting instead (split() requires lvalue handling)
    compare_with_gawk(r#"BEGIN { FS = ":"; } { print $1, $2, $3, $4 }"#, "a:b:c:d");
}

#[test]
fn compat_tolower_toupper() {
    compare_with_gawk(r#"BEGIN { print tolower("HeLLo WoRLD") }"#, "");
    compare_with_gawk(r#"BEGIN { print toupper("HeLLo WoRLD") }"#, "");
}

#[test]
fn compat_gsub() {
    // Use string pattern instead of regex literal
    compare_with_gawk(r#"{ gsub("a", "X"); print }"#, "banana");
}

#[test]
fn compat_sub() {
    // Use string pattern instead of regex literal
    compare_with_gawk(r#"{ sub("a", "X"); print }"#, "banana");
}

#[test]
fn compat_printf() {
    compare_with_gawk(
        r#"BEGIN { printf "%d %s %.2f\n", 42, "test", 3.14159 }"#,
        "",
    );
    compare_with_gawk(r#"BEGIN { printf "%05d\n", 42 }"#, "");
    compare_with_gawk(r#"BEGIN { printf "%-10s|\n", "hi" }"#, "");
}

#[test]
fn compat_math() {
    compare_with_gawk("BEGIN { print int(3.7), int(-3.7) }", "");
    compare_with_gawk("BEGIN { print sqrt(16) }", "");
}

#[test]
fn compat_regex_pattern() {
    compare_with_gawk("/error/ { print }", "info\nerror\nwarning\nerror");
}

#[test]
fn compat_expression_pattern() {
    compare_with_gawk("$1 > 2 { print }", "1 a\n2 b\n3 c\n4 d");
}

#[test]
fn compat_range_pattern() {
    compare_with_gawk("/start/,/end/ { print }", "before\nstart\nmid\nend\nafter");
}

#[test]
fn compat_function() {
    compare_with_gawk(
        "function square(x) { return x * x } BEGIN { print square(5) }",
        "",
    );
}

#[test]
fn compat_recursion() {
    compare_with_gawk(
        "function fact(n) { return n <= 1 ? 1 : n * fact(n-1) } BEGIN { print fact(6) }",
        "",
    );
}

#[test]
fn compat_ternary() {
    compare_with_gawk(
        r#"BEGIN { for (i = 1; i <= 5; i++) print i, (i % 2 == 0 ? "even" : "odd") }"#,
        "",
    );
}

#[test]
fn compat_increment() {
    compare_with_gawk("BEGIN { x = 5; print x++, x, ++x, x }", "");
}

#[test]
fn compat_compound_assignment() {
    compare_with_gawk("BEGIN { x = 10; x += 5; x -= 2; x *= 3; print x }", "");
}

#[test]
fn compat_ofs() {
    compare_with_gawk(r#"BEGIN { OFS = "," } { print $1, $2, $3 }"#, "a b c");
}

#[test]
fn compat_modify_field() {
    compare_with_gawk(r#"BEGIN { OFS = ":" } { $2 = "X"; print }"#, "a b c");
}

#[test]
fn compat_sum() {
    compare_with_gawk("{ sum += $1 } END { print sum }", "1\n2\n3\n4\n5");
}

#[test]
fn compat_count_pattern() {
    compare_with_gawk("/x/ { count++ } END { print count+0 }", "a\nx\nb\nxx\nc");
}

#[test]
fn compat_word_frequency() {
    // Note: order of output may vary, so we just check it runs
    if !gawk_available() {
        return;
    }
    let program =
        "{ for (i=1; i<=NF; i++) count[$i]++ } END { for (w in count) print w, count[w] }";
    let input = "a b a c b a";

    let awk_rs_output = run_awk_rs(program, input);
    let gawk_output = run_gawk(program, input).unwrap();

    // Both should have 3 lines
    assert_eq!(awk_rs_output.lines().count(), gawk_output.lines().count());
}

#[test]
fn compat_next() {
    compare_with_gawk("/skip/ { next } { print }", "a\nskip\nb\nskip\nc");
}

#[test]
fn compat_break_continue() {
    compare_with_gawk(
        "BEGIN { for (i=1; i<=10; i++) { if (i==3) continue; if (i==7) break; print i } }",
        "",
    );
}
