# awk-rs Benchmarks

This document describes the benchmarking infrastructure and performance characteristics of awk-rs.

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark group
cargo bench -- lexer
cargo bench -- parser
cargo bench -- interpreter
cargo bench -- throughput
cargo bench -- builtins

# Generate HTML reports (saved in target/criterion/)
cargo bench
open target/criterion/report/index.html
```

## Benchmark Categories

### 1. Lexer Benchmarks (`lexer/`)

Measures tokenization performance.

| Benchmark | Description | Typical Time |
|-----------|-------------|--------------|
| `simple_program` | Tokenize `BEGIN { print "hello" }` | ~90 ns |
| `complex_program` | Tokenize multi-rule program with loops, conditionals | ~1.7 µs |

### 2. Parser Benchmarks (`parser/`)

Measures AST construction performance.

| Benchmark | Description | Typical Time |
|-----------|-------------|--------------|
| `parse_program` | Parse program with function, loops, patterns | ~4.5 µs |

### 3. Interpreter Benchmarks (`interpreter/`)

Measures execution performance of various AWK constructs.

| Benchmark | Description | Typical Time |
|-----------|-------------|--------------|
| `arithmetic` | 1000 iterations of arithmetic ops | ~195 µs |
| `string_concat` | 100 string concatenations | ~28 µs |
| `field_access` | Access $1, $5, $10 from 10-field record | ~1.5 µs |
| `pattern_matching` | Match /error/ against 100 lines | ~17 µs |
| `array_operations` | 100 array insertions + iteration | ~45 µs |
| `printf_formatting` | 100 printf calls with formatting | ~120 µs |

### 4. Throughput Benchmarks (`throughput/`)

Measures end-to-end processing speed.

| Input Size | Operation | Throughput |
|------------|-----------|------------|
| 100 lines | Sum column | ~2 MB/s |
| 1000 lines | Sum column | ~4 MB/s |
| 10000 lines | Sum column | ~5 MB/s |

### 5. Built-in Functions (`builtins/`)

Measures individual built-in function performance.

| Function | Iterations | Typical Time |
|----------|------------|--------------|
| `length` | 1000 | ~25 µs |
| `substr` | 1000 | ~35 µs |
| `index` | 1000 | ~30 µs |
| `split` | 100 | ~15 µs |
| `gsub` | 100 | ~50 µs |
| `sprintf` | 1000 | ~200 µs |
| `math_functions` | 1000 | ~80 µs |

## Performance Optimizations

### Implemented Optimizations

1. **Pre-allocated Token Vector**
   - Estimate token count based on source length
   - Reduces reallocations during lexing

2. **Optimized Field Splitting**
   - Byte-based splitting for single-character separators
   - Pre-estimate field count to reduce vector reallocations
   - Cache compiled regex patterns for multi-char separators

3. **Value System Optimizations**
   - Fast integer-to-string conversion (`itoa_fast`)
   - Byte-based number parsing (avoid char-by-char iteration)
   - `Cow<str>` for zero-copy string access where possible
   - Inline hints on hot path functions

4. **Regex Caching**
   - All regex patterns are compiled once and cached
   - Avoids recompilation on each pattern match

5. **Release Profile Optimizations**
   - LTO (Link-Time Optimization) enabled
   - Single codegen unit for better optimization
   - Panic = abort (smaller binary, faster panics)
   - Maximum optimization level (`opt-level = 3`)

### Potential Future Optimizations

1. **SIMD for Field Splitting**
   - Use SIMD instructions for finding field separators
   - Particularly beneficial for single-character FS

2. **String Interning**
   - Intern common strings (field separator, variable names)
   - Reduce allocation for repeated strings

3. **Arena Allocation**
   - Use arena allocator for AST nodes
   - Reduce allocation overhead during parsing

4. **Bytecode Compilation**
   - Compile AST to bytecode for faster interpretation
   - Would benefit programs processing many records

5. **JIT Compilation**
   - JIT compile hot loops using Cranelift
   - Only beneficial for very large inputs

## Profiling

### Using perf (Linux)

```bash
# Build with debug symbols
cargo build --release

# Profile a specific workload
perf record ./target/release/awk-rs 'your program' input.txt
perf report

# Flame graph
perf record -g ./target/release/awk-rs 'program' input.txt
perf script | stackcollapse-perf.pl | flamegraph.pl > flame.svg
```

### Using Instruments (macOS)

```bash
cargo build --release
instruments -t "Time Profiler" ./target/release/awk-rs 'program' input.txt
```

### Using cargo-flamegraph

```bash
cargo install flamegraph
cargo flamegraph --bench benchmarks -- --bench
```

## Comparison with gawk

To compare performance with GNU AWK:

```bash
# Generate test data
seq 1 100000 | awk '{print $1, $1*2, $1*3}' > /tmp/test.txt

# Benchmark gawk
time gawk '{ sum += $1 } END { print sum }' /tmp/test.txt

# Benchmark awk-rs
time ./target/release/awk-rs '{ sum += $1 } END { print sum }' /tmp/test.txt
```

Typical results (100,000 lines):
- gawk: ~50ms
- awk-rs: ~80ms (current, improving)

## Memory Usage

awk-rs aims for reasonable memory usage:

- Strings are stored once and referenced where possible
- Fields are only materialized when accessed
- Regex patterns are cached but bounded

To profile memory:

```bash
# Using heaptrack (Linux)
heaptrack ./target/release/awk-rs 'program' input.txt
heaptrack_gui heaptrack.awk-rs.*.gz

# Using Instruments (macOS)
instruments -t "Allocations" ./target/release/awk-rs 'program' input.txt
```

## Continuous Benchmarking

Consider setting up continuous benchmarking with:

1. **GitHub Actions + Bencher**
   - Track benchmark results over time
   - Alert on performance regressions

2. **criterion-compare**
   ```bash
   cargo install critcmp
   cargo bench -- --save-baseline main
   # Make changes
   cargo bench -- --save-baseline feature
   critcmp main feature
   ```
