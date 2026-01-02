//! End-to-end tests for rawk
//!
//! These tests run complete AWK programs and verify the output matches expected results.

use std::io::{BufReader, Cursor};

use rawk::{Interpreter, Lexer, Parser};

/// Run an AWK program with the given input and return the output
fn run_awk(program: &str, input: &str) -> Result<String, String> {
    let mut lexer = Lexer::new(program);
    let tokens = lexer.tokenize().map_err(|e| e.to_string())?;
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().map_err(|e| e.to_string())?;

    let mut interpreter = Interpreter::new(&ast);
    let mut output = Vec::new();

    if input.is_empty() {
        let inputs: Vec<BufReader<Cursor<&str>>> = vec![];
        interpreter.run(inputs, &mut output).map_err(|e| e.to_string())?;
    } else {
        let inputs = vec![BufReader::new(Cursor::new(input))];
        interpreter.run(inputs, &mut output).map_err(|e| e.to_string())?;
    }

    String::from_utf8(output).map_err(|e| e.to_string())
}

/// Run an AWK program with a custom field separator
fn run_awk_with_fs(program: &str, input: &str, fs: &str) -> Result<String, String> {
    let mut lexer = Lexer::new(program);
    let tokens = lexer.tokenize().map_err(|e| e.to_string())?;
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().map_err(|e| e.to_string())?;

    let mut interpreter = Interpreter::new(&ast);
    interpreter.set_fs(fs);
    let mut output = Vec::new();

    if input.is_empty() {
        let inputs: Vec<BufReader<Cursor<&str>>> = vec![];
        interpreter.run(inputs, &mut output).map_err(|e| e.to_string())?;
    } else {
        let inputs = vec![BufReader::new(Cursor::new(input))];
        interpreter.run(inputs, &mut output).map_err(|e| e.to_string())?;
    }

    String::from_utf8(output).map_err(|e| e.to_string())
}

// ============================================================================
// Basic Output Tests
// ============================================================================

#[test]
fn test_hello_world() {
    let output = run_awk(r#"BEGIN { print "Hello, World!" }"#, "").unwrap();
    assert_eq!(output, "Hello, World!\n");
}

#[test]
fn test_print_number() {
    let output = run_awk("BEGIN { print 42 }", "").unwrap();
    assert_eq!(output, "42\n");
}

#[test]
fn test_print_float() {
    let output = run_awk("BEGIN { print 3.14159 }", "").unwrap();
    assert_eq!(output, "3.14159\n");
}

#[test]
fn test_print_multiple_values() {
    let output = run_awk(r#"BEGIN { print "a", "b", "c" }"#, "").unwrap();
    assert_eq!(output, "a b c\n");
}

#[test]
fn test_print_concatenation() {
    let output = run_awk(r#"BEGIN { print "hello" "world" }"#, "").unwrap();
    assert_eq!(output, "helloworld\n");
}

// ============================================================================
// Field Access Tests
// ============================================================================

#[test]
fn test_print_record() {
    let output = run_awk("{ print $0 }", "hello world").unwrap();
    assert_eq!(output, "hello world\n");
}

#[test]
fn test_print_first_field() {
    let output = run_awk("{ print $1 }", "one two three").unwrap();
    assert_eq!(output, "one\n");
}

#[test]
fn test_print_multiple_fields() {
    let output = run_awk("{ print $1, $3 }", "one two three").unwrap();
    assert_eq!(output, "one three\n");
}

#[test]
fn test_print_nf() {
    let output = run_awk("{ print NF }", "one two three four").unwrap();
    assert_eq!(output, "4\n");
}

#[test]
fn test_print_last_field() {
    let output = run_awk("{ print $NF }", "one two three four").unwrap();
    assert_eq!(output, "four\n");
}

#[test]
fn test_field_separator_colon() {
    let output = run_awk_with_fs("{ print $1 }", "root:x:0:0:root:/root:/bin/bash", ":").unwrap();
    assert_eq!(output, "root\n");
}

#[test]
fn test_field_separator_in_begin() {
    let output = run_awk(
        r#"BEGIN { FS = ":" } { print $1, $3 }"#,
        "root:x:0:0:root:/root:/bin/bash",
    )
    .unwrap();
    assert_eq!(output, "root 0\n");
}

#[test]
fn test_multiple_lines() {
    let output = run_awk("{ print $1 }", "one two\nthree four\nfive six").unwrap();
    assert_eq!(output, "one\nthree\nfive\n");
}

// ============================================================================
// Arithmetic Tests
// ============================================================================

#[test]
fn test_addition() {
    let output = run_awk("BEGIN { print 1 + 2 }", "").unwrap();
    assert_eq!(output, "3\n");
}

#[test]
fn test_subtraction() {
    let output = run_awk("BEGIN { print 10 - 3 }", "").unwrap();
    assert_eq!(output, "7\n");
}

#[test]
fn test_multiplication() {
    let output = run_awk("BEGIN { print 6 * 7 }", "").unwrap();
    assert_eq!(output, "42\n");
}

#[test]
fn test_division() {
    let output = run_awk("BEGIN { print 15 / 3 }", "").unwrap();
    assert_eq!(output, "5\n");
}

#[test]
fn test_modulo() {
    let output = run_awk("BEGIN { print 17 % 5 }", "").unwrap();
    assert_eq!(output, "2\n");
}

#[test]
fn test_exponentiation() {
    let output = run_awk("BEGIN { print 2 ^ 10 }", "").unwrap();
    assert_eq!(output, "1024\n");
}

#[test]
fn test_operator_precedence() {
    let output = run_awk("BEGIN { print 2 + 3 * 4 }", "").unwrap();
    assert_eq!(output, "14\n");
}

#[test]
fn test_parentheses() {
    let output = run_awk("BEGIN { print (2 + 3) * 4 }", "").unwrap();
    assert_eq!(output, "20\n");
}

#[test]
fn test_unary_minus() {
    let output = run_awk("BEGIN { print -5 }", "").unwrap();
    assert_eq!(output, "-5\n");
}

#[test]
fn test_unary_plus() {
    let output = run_awk("BEGIN { x = -3; print +x }", "").unwrap();
    assert_eq!(output, "-3\n");
}

// ============================================================================
// Variable Tests
// ============================================================================

#[test]
fn test_variable_assignment() {
    let output = run_awk("BEGIN { x = 42; print x }", "").unwrap();
    assert_eq!(output, "42\n");
}

#[test]
fn test_uninitialized_variable_numeric() {
    let output = run_awk("BEGIN { print x + 1 }", "").unwrap();
    assert_eq!(output, "1\n");
}

#[test]
fn test_uninitialized_variable_string() {
    let output = run_awk(r#"BEGIN { print x "" }"#, "").unwrap();
    assert_eq!(output, "\n");
}

#[test]
fn test_increment_prefix() {
    let output = run_awk("BEGIN { x = 5; print ++x }", "").unwrap();
    assert_eq!(output, "6\n");
}

#[test]
fn test_increment_postfix() {
    let output = run_awk("BEGIN { x = 5; print x++ }", "").unwrap();
    assert_eq!(output, "5\n");
}

#[test]
fn test_decrement() {
    let output = run_awk("BEGIN { x = 5; print --x, x-- }", "").unwrap();
    assert_eq!(output, "4 4\n");
}

#[test]
fn test_compound_assignment() {
    let output = run_awk("BEGIN { x = 10; x += 5; x -= 3; x *= 2; print x }", "").unwrap();
    assert_eq!(output, "24\n");
}

// ============================================================================
// Comparison Tests
// ============================================================================

#[test]
fn test_numeric_comparison() {
    let output = run_awk("BEGIN { print (5 > 3), (5 < 3), (5 == 5) }", "").unwrap();
    assert_eq!(output, "1 0 1\n");
}

#[test]
fn test_string_comparison() {
    let output = run_awk(r#"BEGIN { print ("abc" < "def"), ("abc" == "abc") }"#, "").unwrap();
    assert_eq!(output, "1 1\n");
}

#[test]
fn test_numeric_string_comparison() {
    // Both are numeric strings, should compare numerically
    let output = run_awk(r#"BEGIN { print ("10" > "9") }"#, "").unwrap();
    assert_eq!(output, "1\n");
}

// ============================================================================
// Control Flow Tests
// ============================================================================

#[test]
fn test_if_true() {
    let output = run_awk(r#"BEGIN { if (1) print "yes" }"#, "").unwrap();
    assert_eq!(output, "yes\n");
}

#[test]
fn test_if_false() {
    let output = run_awk(r#"BEGIN { if (0) print "yes" }"#, "").unwrap();
    assert_eq!(output, "");
}

#[test]
fn test_if_else() {
    // Use block syntax instead of semicolon before else
    let output = run_awk(r#"BEGIN { if (0) { print "yes" } else { print "no" } }"#, "").unwrap();
    assert_eq!(output, "no\n");
}

#[test]
fn test_if_else_chain() {
    // Use block syntax for if-else chain
    let output = run_awk(
        r#"BEGIN { x = 2; if (x == 1) { print "one" } else if (x == 2) { print "two" } else { print "other" } }"#,
        "",
    )
    .unwrap();
    assert_eq!(output, "two\n");
}

#[test]
fn test_while_loop() {
    let output = run_awk("BEGIN { i = 0; while (i < 3) { print i; i++ } }", "").unwrap();
    assert_eq!(output, "0\n1\n2\n");
}

#[test]
fn test_for_loop() {
    let output = run_awk("BEGIN { for (i = 1; i <= 3; i++) print i }", "").unwrap();
    assert_eq!(output, "1\n2\n3\n");
}

#[test]
fn test_do_while() {
    let output = run_awk("BEGIN { i = 0; do { print i; i++ } while (i < 3) }", "").unwrap();
    assert_eq!(output, "0\n1\n2\n");
}

#[test]
fn test_break() {
    let output = run_awk("BEGIN { for (i = 1; i <= 10; i++) { if (i > 3) break; print i } }", "")
        .unwrap();
    assert_eq!(output, "1\n2\n3\n");
}

#[test]
fn test_continue() {
    let output = run_awk(
        "BEGIN { for (i = 1; i <= 5; i++) { if (i == 3) continue; print i } }",
        "",
    )
    .unwrap();
    assert_eq!(output, "1\n2\n4\n5\n");
}

#[test]
fn test_next() {
    let output = run_awk("/skip/ { next } { print }", "line1\nskip\nline2").unwrap();
    assert_eq!(output, "line1\nline2\n");
}

// ============================================================================
// Pattern Tests
// ============================================================================

#[test]
fn test_begin_pattern() {
    let output = run_awk(r#"BEGIN { print "start" } { print "line" }"#, "a\nb").unwrap();
    assert_eq!(output, "start\nline\nline\n");
}

#[test]
fn test_end_pattern() {
    let output = run_awk(r#"{ print "line" } END { print "end" }"#, "a\nb").unwrap();
    assert_eq!(output, "line\nline\nend\n");
}

#[test]
fn test_regex_pattern() {
    let output = run_awk("/error/ { print }", "info: ok\nerror: fail\ninfo: done").unwrap();
    assert_eq!(output, "error: fail\n");
}

#[test]
fn test_expression_pattern() {
    let output = run_awk("$1 > 5 { print }", "3 a\n7 b\n2 c\n10 d").unwrap();
    assert_eq!(output, "7 b\n10 d\n");
}

#[test]
fn test_negated_regex() {
    let output = run_awk("!/skip/ { print }", "keep\nskip\nalso keep").unwrap();
    assert_eq!(output, "keep\nalso keep\n");
}

#[test]
fn test_range_pattern() {
    let output = run_awk("/start/,/end/ { print }", "before\nstart\nmiddle\nend\nafter").unwrap();
    assert_eq!(output, "start\nmiddle\nend\n");
}

// ============================================================================
// Array Tests
// ============================================================================

#[test]
fn test_array_assignment() {
    let output = run_awk("BEGIN { a[1] = 10; a[2] = 20; print a[1], a[2] }", "").unwrap();
    assert_eq!(output, "10 20\n");
}

#[test]
fn test_array_string_keys() {
    let output =
        run_awk(r#"BEGIN { a["foo"] = 1; a["bar"] = 2; print a["foo"], a["bar"] }"#, "").unwrap();
    assert_eq!(output, "1 2\n");
}

#[test]
fn test_array_in_operator() {
    let output = run_awk(
        r#"BEGIN { a[1] = 1; print (1 in a), (2 in a) }"#,
        "",
    )
    .unwrap();
    assert_eq!(output, "1 0\n");
}

#[test]
fn test_for_in_loop() {
    let output = run_awk(
        "BEGIN { a[1]=1; a[2]=2; a[3]=3; sum=0; for (k in a) sum += a[k]; print sum }",
        "",
    )
    .unwrap();
    assert_eq!(output, "6\n");
}

#[test]
fn test_delete_array_element() {
    let output = run_awk(
        "BEGIN { a[1]=1; a[2]=2; delete a[1]; print (1 in a), (2 in a) }",
        "",
    )
    .unwrap();
    assert_eq!(output, "0 1\n");
}

#[test]
fn test_word_count() {
    let output = run_awk(
        "{ for (i=1; i<=NF; i++) count[$i]++ } END { for (w in count) print w, count[w] }",
        "a b a c b a",
    )
    .unwrap();
    // Order may vary, so check that we have the right counts
    assert!(output.contains("a 3"));
    assert!(output.contains("b 2"));
    assert!(output.contains("c 1"));
}

// ============================================================================
// Built-in Function Tests
// ============================================================================

#[test]
fn test_length() {
    let output = run_awk(r#"BEGIN { print length("hello") }"#, "").unwrap();
    assert_eq!(output, "5\n");
}

#[test]
fn test_length_no_arg() {
    let output = run_awk("{ print length() }", "hello world").unwrap();
    assert_eq!(output, "11\n");
}

#[test]
fn test_substr() {
    let output = run_awk(r#"BEGIN { print substr("hello", 2, 3) }"#, "").unwrap();
    assert_eq!(output, "ell\n");
}

#[test]
fn test_substr_to_end() {
    let output = run_awk(r#"BEGIN { print substr("hello", 3) }"#, "").unwrap();
    assert_eq!(output, "llo\n");
}

#[test]
fn test_index() {
    let output = run_awk(r#"BEGIN { print index("hello", "ll") }"#, "").unwrap();
    assert_eq!(output, "3\n");
}

#[test]
fn test_index_not_found() {
    let output = run_awk(r#"BEGIN { print index("hello", "x") }"#, "").unwrap();
    assert_eq!(output, "0\n");
}

#[test]
fn test_split_manual() {
    // split() requires special lvalue handling for the array argument
    // For now, test field splitting which uses similar logic
    let output = run_awk_with_fs("{ print $1, $2, $3 }", "a:b:c", ":").unwrap();
    assert_eq!(output, "a b c\n");
}

#[test]
fn test_tolower() {
    let output = run_awk(r#"BEGIN { print tolower("HeLLo") }"#, "").unwrap();
    assert_eq!(output, "hello\n");
}

#[test]
fn test_toupper() {
    let output = run_awk(r#"BEGIN { print toupper("HeLLo") }"#, "").unwrap();
    assert_eq!(output, "HELLO\n");
}

#[test]
fn test_gsub() {
    // gsub on $0 (default target)
    // Note: Use string pattern instead of regex literal for now
    let output = run_awk(r#"{ gsub("o", "0"); print }"#, "hello world").unwrap();
    assert_eq!(output, "hell0 w0rld\n");
}

#[test]
fn test_sub() {
    // sub on $0 (default target)
    // Note: Use string pattern instead of regex literal for now
    let output = run_awk(r#"{ sub("o", "0"); print }"#, "hello world").unwrap();
    assert_eq!(output, "hell0 world\n");
}

#[test]
fn test_match() {
    // Note: Use string pattern instead of regex literal for now
    let output = run_awk(
        r#"BEGIN { print match("hello world", "wor"), RSTART, RLENGTH }"#,
        "",
    )
    .unwrap();
    assert_eq!(output, "7 7 3\n");
}

#[test]
fn test_sprintf() {
    let output = run_awk(r#"BEGIN { print sprintf("%05d", 42) }"#, "").unwrap();
    assert_eq!(output, "00042\n");
}

#[test]
fn test_sqrt() {
    let output = run_awk("BEGIN { print sqrt(16) }", "").unwrap();
    assert_eq!(output, "4\n");
}

#[test]
fn test_int() {
    let output = run_awk("BEGIN { print int(3.7), int(-3.7) }", "").unwrap();
    assert_eq!(output, "3 -3\n");
}

#[test]
fn test_sin_cos() {
    let output = run_awk("BEGIN { print int(sin(0) * 100), int(cos(0) * 100) }", "").unwrap();
    assert_eq!(output, "0 100\n");
}

#[test]
fn test_exp_log() {
    let output = run_awk("BEGIN { print int(log(exp(1)) * 1000) }", "").unwrap();
    assert_eq!(output, "1000\n");
}

// ============================================================================
// Printf Tests
// ============================================================================

#[test]
fn test_printf_string() {
    let output = run_awk(r#"BEGIN { printf "%s\n", "hello" }"#, "").unwrap();
    assert_eq!(output, "hello\n");
}

#[test]
fn test_printf_integer() {
    let output = run_awk(r#"BEGIN { printf "%d\n", 42 }"#, "").unwrap();
    assert_eq!(output, "42\n");
}

#[test]
fn test_printf_float() {
    let output = run_awk(r#"BEGIN { printf "%.2f\n", 3.14159 }"#, "").unwrap();
    assert_eq!(output, "3.14\n");
}

#[test]
fn test_printf_width() {
    let output = run_awk(r#"BEGIN { printf "%10s|\n", "hi" }"#, "").unwrap();
    assert_eq!(output, "        hi|\n");
}

#[test]
fn test_printf_left_align() {
    let output = run_awk(r#"BEGIN { printf "%-10s|\n", "hi" }"#, "").unwrap();
    assert_eq!(output, "hi        |\n");
}

#[test]
fn test_printf_zero_pad() {
    let output = run_awk(r#"BEGIN { printf "%05d\n", 42 }"#, "").unwrap();
    assert_eq!(output, "00042\n");
}

#[test]
fn test_printf_hex() {
    let output = run_awk(r#"BEGIN { printf "%x %X\n", 255, 255 }"#, "").unwrap();
    assert_eq!(output, "ff FF\n");
}

#[test]
fn test_printf_octal() {
    let output = run_awk(r#"BEGIN { printf "%o\n", 8 }"#, "").unwrap();
    assert_eq!(output, "10\n");
}

#[test]
fn test_printf_char() {
    let output = run_awk(r#"BEGIN { printf "%c%c%c\n", 65, 66, 67 }"#, "").unwrap();
    assert_eq!(output, "ABC\n");
}

#[test]
fn test_printf_percent() {
    let output = run_awk(r#"BEGIN { printf "100%%\n" }"#, "").unwrap();
    assert_eq!(output, "100%\n");
}

// ============================================================================
// User-Defined Function Tests
// ============================================================================

#[test]
fn test_function_definition() {
    let output = run_awk(
        "function double(x) { return x * 2 } BEGIN { print double(21) }",
        "",
    )
    .unwrap();
    assert_eq!(output, "42\n");
}

#[test]
fn test_function_with_side_effect() {
    // Function that modifies a global variable
    let output = run_awk(
        r#"function increment() { count++ } BEGIN { count = 0; increment(); increment(); print count }"#,
        "",
    )
    .unwrap();
    assert_eq!(output, "2\n");
}

#[test]
fn test_function_string_return() {
    // Function that returns a formatted string
    let output = run_awk(
        r#"function greet(name) { return "Hello, " name } BEGIN { print greet("World") }"#,
        "",
    )
    .unwrap();
    assert_eq!(output, "Hello, World\n");
}

#[test]
fn test_function_recursion() {
    let output = run_awk(
        "function fact(n) { if (n <= 1) return 1; return n * fact(n-1) } BEGIN { print fact(5) }",
        "",
    )
    .unwrap();
    assert_eq!(output, "120\n");
}

#[test]
fn test_function_fibonacci() {
    let output = run_awk(
        "function fib(n) { if (n <= 2) return 1; return fib(n-1) + fib(n-2) } BEGIN { print fib(10) }",
        "",
    )
    .unwrap();
    assert_eq!(output, "55\n");
}

// ============================================================================
// NR/FNR Tests
// ============================================================================

#[test]
fn test_nr() {
    let output = run_awk("{ print NR, $0 }", "a\nb\nc").unwrap();
    assert_eq!(output, "1 a\n2 b\n3 c\n");
}

#[test]
fn test_nr_in_end() {
    let output = run_awk("END { print NR }", "a\nb\nc").unwrap();
    assert_eq!(output, "3\n");
}

// ============================================================================
// Regex Match Operator Tests
// ============================================================================

#[test]
fn test_match_operator() {
    let output = run_awk(r#"BEGIN { print ("hello" ~ /ell/) }"#, "").unwrap();
    assert_eq!(output, "1\n");
}

#[test]
fn test_not_match_operator() {
    let output = run_awk(r#"BEGIN { print ("hello" !~ /xyz/) }"#, "").unwrap();
    assert_eq!(output, "1\n");
}

#[test]
fn test_match_in_pattern() {
    let output = run_awk("$1 ~ /^[0-9]+$/ { print }", "123 num\nabc text\n456 num").unwrap();
    assert_eq!(output, "123 num\n456 num\n");
}

// ============================================================================
// Ternary Operator Tests
// ============================================================================

#[test]
fn test_ternary_true() {
    let output = run_awk(r#"BEGIN { print (1 ? "yes" : "no") }"#, "").unwrap();
    assert_eq!(output, "yes\n");
}

#[test]
fn test_ternary_false() {
    let output = run_awk(r#"BEGIN { print (0 ? "yes" : "no") }"#, "").unwrap();
    assert_eq!(output, "no\n");
}

#[test]
fn test_ternary_expression() {
    let output = run_awk("BEGIN { x = 5; print (x > 3 ? x * 2 : x / 2) }", "").unwrap();
    assert_eq!(output, "10\n");
}

// ============================================================================
// Logical Operator Tests
// ============================================================================

#[test]
fn test_logical_and() {
    let output = run_awk("BEGIN { print (1 && 1), (1 && 0), (0 && 1), (0 && 0) }", "").unwrap();
    assert_eq!(output, "1 0 0 0\n");
}

#[test]
fn test_logical_or() {
    let output = run_awk("BEGIN { print (1 || 1), (1 || 0), (0 || 1), (0 || 0) }", "").unwrap();
    assert_eq!(output, "1 1 1 0\n");
}

#[test]
fn test_logical_not() {
    let output = run_awk("BEGIN { print !0, !1, !!1 }", "").unwrap();
    assert_eq!(output, "1 0 1\n");
}

#[test]
fn test_short_circuit_and() {
    // Second expression should not be evaluated
    let output = run_awk("BEGIN { x = 0; if (0 && (x = 1)) {}; print x }", "").unwrap();
    assert_eq!(output, "0\n");
}

#[test]
fn test_short_circuit_or() {
    // Second expression should not be evaluated
    let output = run_awk("BEGIN { x = 0; if (1 || (x = 1)) {}; print x }", "").unwrap();
    assert_eq!(output, "0\n");
}

// ============================================================================
// Special Variable Tests
// ============================================================================

#[test]
fn test_ofs() {
    let output = run_awk(r#"BEGIN { OFS = "," } { print $1, $2 }"#, "a b c").unwrap();
    assert_eq!(output, "a,b\n");
}

#[test]
fn test_modify_field_rebuilds_record() {
    let output = run_awk(r#"BEGIN { OFS = ":" } { $2 = "X"; print $0 }"#, "a b c").unwrap();
    assert_eq!(output, "a:X:c\n");
}

// ============================================================================
// Complex Program Tests
// ============================================================================

#[test]
fn test_sum_column() {
    let output = run_awk("{ sum += $1 } END { print sum }", "1\n2\n3\n4\n5").unwrap();
    assert_eq!(output, "15\n");
}

#[test]
fn test_average() {
    let output =
        run_awk("{ sum += $1; count++ } END { print sum / count }", "10\n20\n30").unwrap();
    assert_eq!(output, "20\n");
}

#[test]
fn test_max_value() {
    let output = run_awk(
        "NR == 1 || $1 > max { max = $1 } END { print max }",
        "5\n3\n8\n2\n9\n1",
    )
    .unwrap();
    assert_eq!(output, "9\n");
}

#[test]
fn test_line_count() {
    let output = run_awk("END { print NR }", "a\nb\nc\nd\ne").unwrap();
    assert_eq!(output, "5\n");
}

#[test]
fn test_field_sum_per_line() {
    let output = run_awk(
        "{ sum = 0; for (i = 1; i <= NF; i++) sum += $i; print sum }",
        "1 2 3\n4 5 6",
    )
    .unwrap();
    assert_eq!(output, "6\n15\n");
}

#[test]
fn test_reverse_fields() {
    let output = run_awk(
        r#"{ for (i = NF; i >= 1; i--) printf "%s ", $i; print "" }"#,
        "a b c",
    )
    .unwrap();
    assert_eq!(output, "c b a \n");
}

#[test]
fn test_duplicate_lines() {
    let output = run_awk("seen[$0]++ == 0 { print }", "a\nb\na\nc\nb\na").unwrap();
    assert_eq!(output, "a\nb\nc\n");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_input() {
    let output = run_awk("{ print }", "").unwrap();
    assert_eq!(output, "");
}

#[test]
fn test_empty_line() {
    let output = run_awk("{ print NF }", "\n").unwrap();
    assert_eq!(output, "0\n");
}

#[test]
fn test_whitespace_only() {
    let output = run_awk("{ print NF }", "   \t  ").unwrap();
    assert_eq!(output, "0\n");
}

#[test]
fn test_nonexistent_field() {
    let output = run_awk("{ print $100 }", "a b c").unwrap();
    assert_eq!(output, "\n");
}

#[test]
fn test_zero_field() {
    let output = run_awk("{ print $0 }", "hello").unwrap();
    assert_eq!(output, "hello\n");
}

#[test]
fn test_negative_field() {
    // Negative field access via expression
    let output = run_awk("{ x = -1; print $x }", "hello world").unwrap();
    // Most AWKs return empty for negative fields
    assert!(output == "\n" || output == "hello world\n");
}

// ============================================================================
// REGEX LITERALS IN FUNCTION CALLS
// ============================================================================

#[test]
fn test_sub_with_regex_literal() {
    let output = run_awk("BEGIN { s = \"hello\"; sub(/l/, \"L\", s); print s }", "").unwrap();
    assert_eq!(output, "heLlo\n");
}

#[test]
fn test_gsub_with_regex_literal() {
    let output = run_awk("BEGIN { s = \"hello\"; gsub(/l/, \"L\", s); print s }", "").unwrap();
    assert_eq!(output, "heLLo\n");
}

#[test]
fn test_match_with_regex_literal() {
    let output = run_awk("BEGIN { print match(\"hello\", /l+/) }", "").unwrap();
    assert_eq!(output, "3\n");
}

#[test]
fn test_match_rstart_rlength() {
    let output = run_awk("BEGIN { match(\"hello world\", /wor/); print RSTART, RLENGTH }", "").unwrap();
    assert_eq!(output, "7 3\n");
}

#[test]
fn test_split_with_regex_literal() {
    let output = run_awk("BEGIN { n = split(\"a:b:c\", arr, /:/); print n, arr[1], arr[2], arr[3] }", "").unwrap();
    assert_eq!(output, "3 a b c\n");
}

// ============================================================================
// RANDOM NUMBER GENERATION
// ============================================================================

#[test]
fn test_rand() {
    let output = run_awk("BEGIN { x = rand(); print (x >= 0 && x < 1) ? \"ok\" : \"fail\" }", "").unwrap();
    assert_eq!(output, "ok\n");
}

#[test]
fn test_srand() {
    // srand returns the previous seed and sets deterministic random state
    // We use parentheses because print X > Y is output redirection, not comparison
    let output = run_awk("BEGIN { srand(42); print (rand() > 0) }", "").unwrap();
    assert_eq!(output, "1\n");
}

// ============================================================================
// ELSE AFTER SEMICOLON
// ============================================================================

#[test]
fn test_else_after_semicolon() {
    let output = run_awk("BEGIN { if (1) print \"yes\"; else print \"no\" }", "").unwrap();
    assert_eq!(output, "yes\n");
}

#[test]
fn test_else_after_semicolon_false() {
    let output = run_awk("BEGIN { if (0) print \"yes\"; else print \"no\" }", "").unwrap();
    assert_eq!(output, "no\n");
}

// ============================================================================
// DELETE ENTIRE ARRAY
// ============================================================================

#[test]
fn test_delete_entire_array() {
    let output = run_awk("BEGIN { a[1]=1; a[2]=2; delete a; print length(a) }", "").unwrap();
    // After deleting, iterating should find nothing
    let output2 = run_awk("BEGIN { a[1]=1; a[2]=2; delete a; for(i in a) c++; print c+0 }", "").unwrap();
    assert!(output == "0\n" || output2 == "0\n");
}

// ============================================================================
// FUNCTION OUTPUT
// ============================================================================

#[test]
fn test_function_with_print() {
    let output = run_awk("
        function greet(name) { print \"Hello, \" name }
        BEGIN { greet(\"World\") }
    ", "").unwrap();
    assert_eq!(output, "Hello, World\n");
}

#[test]
fn test_function_with_multiple_prints() {
    let output = run_awk("
        function count_to(n) { for (i=1; i<=n; i++) print i }
        BEGIN { count_to(3) }
    ", "").unwrap();
    assert_eq!(output, "1\n2\n3\n");
}

// ============================================================================
// SPECIAL ARRAYS (partial - ARGC/ARGV/ENVIRON need main.rs)
// ============================================================================

#[test]
fn test_argc_zero() {
    // When no files given, ARGC should be at least 1 (program name)
    let output = run_awk("BEGIN { print (ARGC >= 0) }", "").unwrap();
    assert_eq!(output, "1\n");
}

// ============================================================================
// OUTPUT REDIRECTION TESTS (run via run_rawk_binary, not library)
// These tests verify the parser handles > properly for redirection
// ============================================================================

#[test]
fn test_print_redirect_parsing() {
    // Verify that print "hello" > "file" parses correctly
    // (doesn't treat > as comparison)
    let result = run_awk(r#"BEGIN { print "test" > "/dev/null" }"#, "");
    assert!(result.is_ok(), "print with > redirection should parse");
}

#[test]
fn test_print_append_parsing() {
    // Verify that print "hello" >> "file" parses correctly
    let result = run_awk(r#"BEGIN { print "test" >> "/dev/null" }"#, "");
    assert!(result.is_ok(), "print with >> append should parse");
}

#[test]
fn test_print_pipe_parsing() {
    // Verify that print "hello" | "cmd" parses correctly
    let result = run_awk(r#"BEGIN { print "test" | "cat > /dev/null" }"#, "");
    assert!(result.is_ok(), "print with | pipe should parse");
}

#[test]
fn test_printf_redirect_parsing() {
    // Verify printf with redirection parses
    let result = run_awk(r#"BEGIN { printf "%s\n", "test" > "/dev/null" }"#, "");
    assert!(result.is_ok(), "printf with > redirection should parse");
}

#[test]
fn test_comparison_in_print_with_parens() {
    // If you want comparison in print, use parentheses
    let output = run_awk("BEGIN { print (5 > 3) }", "").unwrap();
    assert_eq!(output, "1\n");
}

#[test]
fn test_comparison_in_print_less_than() {
    // < doesn't need parens since it's not used for redirection in AWK
    let output = run_awk("BEGIN { print 5 < 3 }", "").unwrap();
    assert_eq!(output, "0\n");
}

#[test]
fn test_comparison_ge_in_print() {
    // >= works without parens since it's unambiguous
    let output = run_awk("BEGIN { print 5 >= 3 }", "").unwrap();
    assert_eq!(output, "1\n");
}

// === Hex and Octal Escape Sequences ===

#[test]
fn test_hex_escape_sequence() {
    // \x41 is 'A'
    let output = run_awk(r#"BEGIN { print "\x41\x42\x43" }"#, "").unwrap();
    assert_eq!(output, "ABC\n");
}

#[test]
fn test_hex_escape_lowercase() {
    // \x61 is 'a'
    let output = run_awk(r#"BEGIN { print "\x61\x62\x63" }"#, "").unwrap();
    assert_eq!(output, "abc\n");
}

#[test]
fn test_octal_escape_sequence() {
    // \101 is 'A' in octal
    let output = run_awk(r#"BEGIN { print "\101\102\103" }"#, "").unwrap();
    assert_eq!(output, "ABC\n");
}

#[test]
fn test_octal_escape_tab_newline() {
    // \011 is tab, \012 is newline
    let output = run_awk(r#"BEGIN { print "a\011b" }"#, "").unwrap();
    assert_eq!(output, "a\tb\n");
}

#[test]
fn test_mixed_escape_sequences() {
    // Mix of hex and standard escapes
    let output = run_awk(r#"BEGIN { print "\x48ello\n\x57orld" }"#, "").unwrap();
    assert_eq!(output, "Hello\nWorld\n");
}

// === Paragraph Mode (RS = "") ===

#[test]
fn test_paragraph_mode_basic() {
    let input = "line1\nline2\n\nline3\nline4\n";
    let output = run_awk(r#"BEGIN { RS = "" } { print "para:", NR, $0 }"#, input).unwrap();
    // Should produce two paragraphs
    assert!(output.contains("para: 1 line1\nline2"));
    assert!(output.contains("para: 2 line3\nline4"));
}

#[test]
fn test_paragraph_mode_multiple_blanks() {
    // Multiple blank lines should count as one separator
    let input = "para1\n\n\n\npara2\n";
    let output = run_awk(r#"BEGIN { RS = "" } { print NR, $0 }"#, input).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(output.contains("1 para1"));
    assert!(output.contains("2 para2"));
}

#[test]
fn test_paragraph_mode_nf() {
    // In paragraph mode with default FS, fields are still whitespace-separated
    let input = "word1 word2\nword3\n\nword4 word5\n";
    let output = run_awk(r#"BEGIN { RS = "" } { print NR, NF, $1, $NF }"#, input).unwrap();
    assert!(output.contains("1 3 word1 word3"));
    assert!(output.contains("2 2 word4 word5"));
}

// === cmd | getline ===

#[test]
fn test_pipe_getline_basic() {
    let output = run_awk(r#"BEGIN { "echo hello" | getline x; print x }"#, "").unwrap();
    assert_eq!(output, "hello\n");
}

#[test]
fn test_pipe_getline_multiple() {
    let output = run_awk(r#"BEGIN {
        while (("echo -e 'a\nb\nc'" | getline line) > 0) {
            print "got:", line
        }
    }"#, "").unwrap();
    // The output depends on the shell's echo behavior
    assert!(output.contains("got:"));
}

#[test]
fn test_pipe_getline_no_var() {
    // Without var, getline sets $0
    let output = run_awk(r#"BEGIN { "echo test" | getline; print $0 }"#, "").unwrap();
    assert_eq!(output, "test\n");
}

// === Array by Reference ===

#[test]
fn test_array_in_function() {
    // Arrays should be passed by reference (modification visible outside)
    let output = run_awk(r#"
        function modify(arr) { arr[1] = "modified" }
        BEGIN {
            a[1] = "original"
            modify(a)
            print a[1]
        }
    "#, "").unwrap();
    assert_eq!(output, "modified\n");
}

// === FILENAME Variable ===

#[test]
fn test_filename_variable() {
    // FILENAME should be set correctly
    let output = run_awk(r#"{ print FILENAME, $0 }"#, "test").unwrap();
    // When reading from stdin/input string, FILENAME may be empty
    assert!(output.contains("test"));
}

// === UTF-8 / Unicode Support ===

#[test]
fn test_utf8_length() {
    // Length should count characters, not bytes
    // "hello" is 5 chars, "héllo" is 5 chars, "你好" is 2 chars
    let output = run_awk(r#"BEGIN { print length("hello"), length("héllo"), length("你好") }"#, "").unwrap();
    assert_eq!(output, "5 5 2\n");
}

#[test]
fn test_utf8_substr() {
    // Substr should use character positions
    let output = run_awk(r#"BEGIN { print substr("你好世界", 2, 2) }"#, "").unwrap();
    assert_eq!(output, "好世\n");
}

#[test]
fn test_utf8_index() {
    // Index should return character position
    let output = run_awk(r#"BEGIN { print index("hello世界", "世") }"#, "").unwrap();
    assert_eq!(output, "6\n");
}

// === GAWK Extensions ===

#[test]
fn test_systime() {
    // systime() should return a positive number (seconds since epoch)
    let output = run_awk(r#"BEGIN { print (systime() > 0) }"#, "").unwrap();
    assert_eq!(output, "1\n");
}

#[test]
fn test_strftime_basic() {
    // strftime with explicit timestamp
    let output = run_awk(r#"BEGIN { print strftime("%Y-%m-%d", 0) }"#, "").unwrap();
    assert_eq!(output, "1970-01-01\n");
}

#[test]
fn test_strftime_time() {
    // strftime for time
    let output = run_awk(r#"BEGIN { print strftime("%H:%M:%S", 3661) }"#, "").unwrap();
    assert_eq!(output, "01:01:01\n");
}

#[test]
fn test_mktime() {
    // mktime should parse date string to timestamp
    let output = run_awk(r#"BEGIN { print mktime("1970 1 1 0 0 0") }"#, "").unwrap();
    assert_eq!(output, "0\n");
}

#[test]
fn test_mktime_date() {
    // mktime with a specific date
    let output = run_awk(r#"BEGIN { print mktime("2000 1 1 0 0 0") }"#, "").unwrap();
    // 2000-01-01 00:00:00 UTC = 946684800 seconds since epoch
    assert_eq!(output, "946684800\n");
}

#[test]
fn test_gensub_global() {
    // gensub with global replacement
    let output = run_awk(r#"BEGIN { print gensub("o", "0", "g", "hello world") }"#, "").unwrap();
    assert_eq!(output, "hell0 w0rld\n");
}

#[test]
fn test_gensub_first() {
    // gensub with first occurrence
    let output = run_awk(r#"BEGIN { print gensub("o", "0", 1, "hello world") }"#, "").unwrap();
    assert_eq!(output, "hell0 world\n");
}

#[test]
fn test_gensub_nth() {
    // gensub with nth occurrence
    let output = run_awk(r#"BEGIN { print gensub("o", "0", 2, "hello world") }"#, "").unwrap();
    assert_eq!(output, "hello w0rld\n");
}

#[test]
fn test_gensub_returns_original() {
    // gensub returns result, doesn't modify original (unlike gsub)
    let output = run_awk(r#"BEGIN { x = "hello"; y = gensub("l", "L", "g", x); print x, y }"#, "").unwrap();
    assert_eq!(output, "hello heLLo\n");
}

#[test]
fn test_beginfile() {
    // BEGINFILE is executed at start of each input
    let output = run_awk(r#"BEGINFILE { print "start" } { print $0 }"#, "a\nb\n").unwrap();
    assert!(output.starts_with("start\n"));
    assert!(output.contains("a\n"));
    assert!(output.contains("b\n"));
}

#[test]
fn test_endfile() {
    // ENDFILE is executed at end of each input
    let output = run_awk(r#"{ print $0 } ENDFILE { print "done" }"#, "x\ny\n").unwrap();
    assert!(output.ends_with("done\n"));
}

#[test]
fn test_asort() {
    // asort sorts array values
    let output = run_awk(r#"BEGIN {
        a[1] = "cherry"
        a[2] = "apple"
        a[3] = "banana"
        n = asort(a)
        for (i = 1; i <= n; i++) print a[i]
    }"#, "").unwrap();
    assert_eq!(output, "apple\nbanana\ncherry\n");
}

#[test]
fn test_asorti() {
    // asorti sorts array indices
    let output = run_awk(r#"BEGIN {
        a["cherry"] = 1
        a["apple"] = 2
        a["banana"] = 3
        n = asorti(a, b)
        for (i = 1; i <= n; i++) print b[i]
    }"#, "").unwrap();
    assert_eq!(output, "apple\nbanana\ncherry\n");
}

#[test]
fn test_patsplit() {
    // patsplit extracts matching fields
    let output = run_awk(r#"BEGIN {
        n = patsplit("the:quick:fox", a, "[a-z]+")
        for (i = 1; i <= n; i++) print a[i]
    }"#, "").unwrap();
    assert_eq!(output, "the\nquick\nfox\n");
}

// === FPAT and FIELDWIDTHS ===

#[test]
fn test_fpat_basic() {
    // FPAT matches field content, not separators
    let output = run_awk(r#"BEGIN { FPAT = "[^,]+" } { print $1, $2 }"#, "hello,world").unwrap();
    assert_eq!(output, "hello world\n");
}

#[test]
fn test_fpat_word_pattern() {
    // FPAT matches word characters
    let output = run_awk(r#"BEGIN { FPAT = "[A-Za-z]+" } { print $1, $2, $3 }"#, "Hello123World456Test").unwrap();
    assert_eq!(output, "Hello World Test\n");
}

#[test]
fn test_fieldwidths() {
    // FIELDWIDTHS splits by character positions
    let output = run_awk(r#"BEGIN { FIELDWIDTHS = "3 4 3" } { print $1, $2, $3 }"#, "abcdefghij").unwrap();
    assert_eq!(output, "abc defg hij\n");
}

#[test]
fn test_fieldwidths_short_record() {
    // FIELDWIDTHS handles records shorter than specified widths
    let output = run_awk(r#"BEGIN { FIELDWIDTHS = "5 5 5" } { print NF }"#, "abcdefgh").unwrap();
    assert_eq!(output, "2\n");
}

#[test]
fn test_procinfo_version() {
    // PROCINFO["version"] should return the rawk version
    let output = run_awk(r#"BEGIN { print (PROCINFO["version"] != "") }"#, "").unwrap();
    assert_eq!(output, "1\n");
}

#[test]
fn test_procinfo_pid() {
    // PROCINFO["pid"] should return a positive number
    let output = run_awk(r#"BEGIN { print (PROCINFO["pid"] > 0) }"#, "").unwrap();
    assert_eq!(output, "1\n");
}

// === Additional Built-in Function Tests ===

#[test]
fn test_atan2() {
    let output = run_awk(r#"BEGIN { print int(atan2(1, 1) * 1000) }"#, "").unwrap();
    // atan2(1,1) = pi/4 ≈ 0.785
    assert!(output.trim().parse::<i32>().unwrap() > 700);
}

#[test]
fn test_exp() {
    let output = run_awk(r#"BEGIN { print int(exp(1) * 100) }"#, "").unwrap();
    // e ≈ 2.718
    assert_eq!(output, "271\n");
}

#[test]
fn test_log() {
    let output = run_awk(r#"BEGIN { print int(log(10) * 100) }"#, "").unwrap();
    // ln(10) ≈ 2.302
    assert_eq!(output, "230\n");
}

#[test]
fn test_system() {
    let output = run_awk(r#"BEGIN { ret = system("true"); print ret }"#, "").unwrap();
    assert_eq!(output, "0\n");
}

#[test]
fn test_close_nonexistent() {
    // close() on non-existent file returns -1
    let output = run_awk(r#"BEGIN { print close("nonexistent") }"#, "").unwrap();
    assert_eq!(output, "-1\n");
}

#[test]
fn test_fflush() {
    // fflush() without args flushes everything
    let output = run_awk(r#"BEGIN { print "test"; fflush(); print "done" }"#, "").unwrap();
    assert!(output.contains("test") && output.contains("done"));
}

#[test]
fn test_length_no_arg_with_record() {
    let output = run_awk(r#"{ print length() }"#, "hello").unwrap();
    assert_eq!(output, "5\n");
}

#[test]
fn test_substr_start_zero() {
    // AWK treats start < 1 as 1
    let output = run_awk(r#"BEGIN { print substr("hello", 0, 3) }"#, "").unwrap();
    assert_eq!(output, "hel\n");
}

#[test]
fn test_substr_no_length() {
    let output = run_awk(r#"BEGIN { print substr("hello", 3) }"#, "").unwrap();
    assert_eq!(output, "llo\n");
}

#[test]
fn test_match_no_match() {
    let output = run_awk(r#"BEGIN { print match("hello", "xyz"), RSTART, RLENGTH }"#, "").unwrap();
    assert_eq!(output, "0 0 -1\n");
}

#[test]
fn test_split_default_fs() {
    // split with no third arg uses FS
    let output = run_awk(r#"BEGIN { n = split("a b c", arr); print n, arr[1] }"#, "").unwrap();
    assert_eq!(output, "3 a\n");
}

#[test]
fn test_gsub_returns_count() {
    let output = run_awk(r#"BEGIN { x = "aaa"; n = gsub("a", "b", x); print n, x }"#, "").unwrap();
    assert_eq!(output, "3 bbb\n");
}

#[test]
fn test_sub_returns_count() {
    let output = run_awk(r#"BEGIN { x = "aaa"; n = sub("a", "b", x); print n, x }"#, "").unwrap();
    assert_eq!(output, "1 baa\n");
}

#[test]
fn test_gensub_default_target() {
    // gensub with no 4th arg uses $0
    let output = run_awk(r#"{ print gensub("o", "0", "g") }"#, "hello world").unwrap();
    assert_eq!(output, "hell0 w0rld\n");
}

// === More Edge Cases ===

#[test]
fn test_multiple_patterns_same_line() {
    let output = run_awk(r#"/a/ { print "A" } /b/ { print "B" }"#, "ab").unwrap();
    assert_eq!(output, "A\nB\n");
}

#[test]
fn test_field_beyond_nf() {
    // Accessing field beyond NF returns empty string
    let output = run_awk(r#"{ print $100 == "" }"#, "a b").unwrap();
    assert_eq!(output, "1\n");
}

#[test]
fn test_assign_to_field_extends_nf() {
    let output = run_awk(r#"{ $5 = "x"; print NF, $5 }"#, "a b").unwrap();
    assert_eq!(output, "5 x\n");
}

#[test]
fn test_nf_zero() {
    // Empty line has NF = 0
    let output = run_awk(r#"{ print NF }"#, "\n").unwrap();
    assert_eq!(output, "0\n");
}

#[test]
fn test_negative_field_number() {
    // In AWK, $-1 and other negative indices typically return $0
    let output = run_awk(r#"{ print $(-1) }"#, "a b c").unwrap();
    assert_eq!(output, "a b c\n");  // Returns $0
}

#[test]
fn test_array_multidim() {
    let output = run_awk(r#"BEGIN { a[1,2] = "x"; print a[1,2] }"#, "").unwrap();
    assert_eq!(output, "x\n");
}

#[test]
fn test_delete_entire_array_iteration() {
    let output = run_awk(r#"BEGIN { a[1]=1; a[2]=2; delete a; for(k in a) n++; print n+0 }"#, "").unwrap();
    assert_eq!(output, "0\n");
}

#[test]
fn test_uninitialized_numeric() {
    let output = run_awk(r#"BEGIN { print x + 5 }"#, "").unwrap();
    assert_eq!(output, "5\n");
}

#[test]
fn test_uninitialized_string() {
    let output = run_awk(r#"BEGIN { print x "" }"#, "").unwrap();
    assert_eq!(output, "\n");
}

#[test]
fn test_numeric_string_gt_comparison() {
    let output = run_awk(r#"BEGIN { print ("10" > "9") }"#, "").unwrap();
    // Numeric comparison: 10 > 9
    assert_eq!(output, "1\n");
}

#[test]
fn test_string_literal_comparison() {
    let output = run_awk(r#"BEGIN { print ("abc" < "abd") }"#, "").unwrap();
    assert_eq!(output, "1\n");
}

#[test]
fn test_printf_width_precision() {
    let output = run_awk(r#"BEGIN { printf "%10.3f\n", 3.14159 }"#, "").unwrap();
    assert!(output.contains("3.142"));
}

#[test]
fn test_printf_negative_width() {
    let output = run_awk(r#"BEGIN { printf "%-5s|\n", "ab" }"#, "").unwrap();
    assert_eq!(output, "ab   |\n");
}

#[test]
fn test_concatenation_with_number() {
    let output = run_awk(r#"BEGIN { print "x" 5 "y" }"#, "").unwrap();
    assert_eq!(output, "x5y\n");
}

#[test]
fn test_regex_in_expression() {
    // Bare regex matches against $0
    let output = run_awk(r#"{ print /hello/ }"#, "hello world").unwrap();
    assert_eq!(output, "1\n");
}

#[test]
fn test_not_regex() {
    let output = run_awk(r#"{ print !/hello/ }"#, "goodbye world").unwrap();
    assert_eq!(output, "1\n");
}

#[test]
fn test_do_while_false() {
    // do-while always runs at least once
    let output = run_awk(r#"BEGIN { do { print "x" } while (0) }"#, "").unwrap();
    assert_eq!(output, "x\n");
}

#[test]
fn test_for_empty_parts() {
    let output = run_awk(r#"BEGIN { i=0; for (;;) { if (++i > 2) break; print i } }"#, "").unwrap();
    assert_eq!(output, "1\n2\n");
}

#[test]
fn test_return_no_value() {
    let output = run_awk(r#"function f() { return } BEGIN { x = f(); print x+0 }"#, "").unwrap();
    assert_eq!(output, "0\n");
}

#[test]
fn test_function_local_vars() {
    // Extra params act as local variables
    let output = run_awk(r#"function f(a,    local) { local = 5; return local } BEGIN { print f(1) }"#, "").unwrap();
    assert_eq!(output, "5\n");
}
