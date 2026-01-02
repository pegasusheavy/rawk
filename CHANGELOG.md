# Changelog

All notable changes to RAWK will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of RAWK - AWK implementation in Rust
- Complete POSIX AWK compatibility for core features
- Full lexer with all AWK token types
- Recursive descent parser with correct operator precedence
- Tree-walking interpreter
- All built-in variables: `$0`, `$n`, `NF`, `NR`, `FNR`, `FS`, `RS`, `OFS`, `ORS`, `FILENAME`, `ARGC`, `ARGV`, `ENVIRON`, `CONVFMT`, `OFMT`, `RSTART`, `RLENGTH`, `SUBSEP`
- String functions: `length`, `substr`, `index`, `split`, `sub`, `gsub`, `match`, `sprintf`, `tolower`, `toupper`
- Math functions: `sin`, `cos`, `atan2`, `exp`, `log`, `sqrt`, `int`, `rand`, `srand`
- I/O functions: `print`, `printf`, `getline`, `close`, `fflush`, `system`
- Output redirection: `>`, `>>`, `|`
- User-defined functions with local scope and recursion
- Associative arrays with multi-dimensional subscripts
- All control flow: `if`/`else`, `while`, `for`, `for-in`, `do-while`, `break`, `continue`, `next`, `nextfile`, `exit`, `return`
- Pattern types: expression, regex, range, `BEGIN`, `END`, compound (`&&`, `||`, `!`)
- Regex matching with POSIX ERE via `regex` crate
- `delete` for array elements and entire arrays
- CLI options: `-F`, `-f`, `-v`, `--help`, `--version`
- Comprehensive test suite (195 tests)
- gawk compatibility tests
- Criterion benchmarks
- Fuzzing targets

### Performance
- Optimized field splitting (byte-based for single-char FS)
- Compiled regex caching
- Release profile with LTO and single codegen unit

## [0.1.0] - 2026-01-02

Initial development release.

---

## Version History

- **0.1.0**: Initial release with core AWK functionality
