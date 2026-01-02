use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::io::{BufReader, Cursor};

use rawk::{Interpreter, Lexer, Parser};

fn run_awk(program: &str, input: &str) -> String {
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

// ============ Lexer Benchmarks ============

fn bench_lexer(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer");

    // Simple program
    let simple = r#"BEGIN { print "hello" }"#;
    group.bench_function("simple_program", |b| {
        b.iter(|| {
            let mut lexer = Lexer::new(black_box(simple));
            lexer.tokenize().unwrap()
        })
    });

    // Complex program with many tokens
    let complex = r#"
        BEGIN {
            FS = ":"
            count = 0
        }
        /pattern/ {
            for (i = 1; i <= NF; i++) {
                if ($i ~ /[0-9]+/) {
                    sum += $i
                    count++
                }
            }
        }
        END {
            if (count > 0) {
                printf "Average: %.2f\n", sum / count
            }
        }
    "#;
    group.bench_function("complex_program", |b| {
        b.iter(|| {
            let mut lexer = Lexer::new(black_box(complex));
            lexer.tokenize().unwrap()
        })
    });

    group.finish();
}

// ============ Parser Benchmarks ============

fn bench_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");

    let program = r#"
        function factorial(n) {
            if (n <= 1) return 1
            return n * factorial(n - 1)
        }
        BEGIN {
            for (i = 1; i <= 10; i++) {
                print i, factorial(i)
            }
        }
        { sum += $1 }
        /error/ { errors++ }
        END { print "Total:", sum, "Errors:", errors }
    "#;

    // Pre-tokenize for parser benchmark
    let mut lexer = Lexer::new(program);
    let tokens = lexer.tokenize().unwrap();

    group.bench_function("parse_program", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(tokens.clone()));
            parser.parse().unwrap()
        })
    });

    group.finish();
}

// ============ Interpreter Benchmarks ============

fn bench_interpreter(c: &mut Criterion) {
    let mut group = c.benchmark_group("interpreter");

    // Arithmetic operations
    group.bench_function("arithmetic", |b| {
        b.iter(|| {
            run_awk(
                black_box("BEGIN { x = 0; for (i = 1; i <= 1000; i++) x += i * 2 - 1; print x }"),
                "",
            )
        })
    });

    // String operations
    group.bench_function("string_concat", |b| {
        b.iter(|| {
            run_awk(
                black_box(
                    r#"BEGIN { s = ""; for (i = 1; i <= 100; i++) s = s "x"; print length(s) }"#,
                ),
                "",
            )
        })
    });

    // Field splitting
    let input_line = "field1 field2 field3 field4 field5 field6 field7 field8 field9 field10";
    group.bench_function("field_access", |b| {
        b.iter(|| run_awk(black_box("{ print $1, $5, $10 }"), black_box(input_line)))
    });

    // Pattern matching
    let pattern_input = (0..100)
        .map(|i| {
            if i % 10 == 0 {
                format!("error line {}", i)
            } else {
                format!("normal line {}", i)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    group.bench_function("pattern_matching", |b| {
        b.iter(|| {
            run_awk(
                black_box("/error/ { count++ } END { print count }"),
                black_box(&pattern_input),
            )
        })
    });

    // Array operations
    group.bench_function("array_operations", |b| {
        b.iter(|| {
            run_awk(
                black_box("BEGIN { for (i = 1; i <= 100; i++) arr[i] = i * 2; for (k in arr) sum += arr[k]; print sum }"),
                ""
            )
        })
    });

    // Printf formatting
    group.bench_function("printf_formatting", |b| {
        b.iter(|| {
            run_awk(
                black_box(r#"BEGIN { for (i = 1; i <= 100; i++) printf "%05d: %-20s %8.2f\n", i, "test", i * 3.14 }"#),
                ""
            )
        })
    });

    group.finish();
}

// ============ End-to-End Benchmarks ============

fn bench_e2e_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");

    // Generate input of various sizes
    for size in [100, 1000, 10000] {
        let input: String = (0..size)
            .map(|i| format!("{} {} {} {}", i, i * 2, i * 3, i % 100))
            .collect::<Vec<_>>()
            .join("\n");

        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("sum_column", size), &input, |b, input| {
            b.iter(|| {
                run_awk(
                    black_box("{ sum += $1 } END { print sum }"),
                    black_box(input),
                )
            })
        });
    }

    group.finish();
}

fn bench_builtin_functions(c: &mut Criterion) {
    let mut group = c.benchmark_group("builtins");

    group.bench_function("length", |b| {
        b.iter(|| {
            run_awk(
                black_box(
                    r#"BEGIN { s = "hello world"; for (i = 1; i <= 1000; i++) x += length(s) }"#,
                ),
                "",
            )
        })
    });

    group.bench_function("substr", |b| {
        b.iter(|| {
            run_awk(black_box(r#"BEGIN { s = "hello world test string"; for (i = 1; i <= 1000; i++) x = substr(s, 5, 10) }"#), "")
        })
    });

    group.bench_function("index", |b| {
        b.iter(|| {
            run_awk(black_box(r#"BEGIN { s = "hello world test string"; for (i = 1; i <= 1000; i++) x = index(s, "test") }"#), "")
        })
    });

    group.bench_function("split", |b| {
        b.iter(|| {
            run_awk(black_box(r#"BEGIN { s = "a:b:c:d:e:f:g:h:i:j"; for (i = 1; i <= 100; i++) n = split(s, arr, ":") }"#), "")
        })
    });

    group.bench_function("gsub", |b| {
        b.iter(|| {
            run_awk(black_box(r#"BEGIN { s = "hello world hello world"; for (i = 1; i <= 100; i++) { t = s; gsub(/hello/, "hi", t) } }"#), "")
        })
    });

    group.bench_function("sprintf", |b| {
        b.iter(|| {
            run_awk(black_box(r#"BEGIN { for (i = 1; i <= 1000; i++) s = sprintf("%d %.2f %s", i, i * 3.14, "test") }"#), "")
        })
    });

    group.bench_function("math_functions", |b| {
        b.iter(|| {
            run_awk(
                black_box("BEGIN { for (i = 1; i <= 1000; i++) x = sin(i) + cos(i) + sqrt(i) }"),
                "",
            )
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_lexer,
    bench_parser,
    bench_interpreter,
    bench_e2e_throughput,
    bench_builtin_functions,
);

criterion_main!(benches);
