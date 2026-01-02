mod builtins;
mod expr;
pub mod stmt;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::fs::File;
use std::process::{Child, ChildStdout, ChildStdin};

use crate::ast::*;
use crate::error::{Error, Result};
use crate::value::Value;

use regex::Regex;

/// Input source for getline from pipe
pub struct PipeInput {
    #[allow(dead_code)]
    child: Child,
    reader: BufReader<ChildStdout>,
}

/// Output destination for print/printf redirection
pub enum OutputFile {
    File(File),
    Pipe(ChildStdin),
}

impl Write for OutputFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            OutputFile::File(f) => f.write(buf),
            OutputFile::Pipe(p) => p.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            OutputFile::File(f) => f.flush(),
            OutputFile::Pipe(p) => p.flush(),
        }
    }
}

/// The AWK interpreter runtime
pub struct Interpreter<'a> {
    /// The parsed program
    program: &'a Program,

    /// Global variables
    pub(crate) variables: HashMap<String, Value>,

    /// Associative arrays
    pub(crate) arrays: HashMap<String, HashMap<String, Value>>,

    /// User-defined functions
    pub(crate) functions: HashMap<String, &'a FunctionDef>,

    /// Built-in variables
    /// Field separator (FS)
    pub(crate) fs: String,
    /// Output field separator (OFS)
    pub(crate) ofs: String,
    /// Record separator (RS)
    pub(crate) rs: String,
    /// Output record separator (ORS)
    pub(crate) ors: String,
    /// Number format for output (OFMT)
    pub(crate) ofmt: String,
    /// Conversion format (CONVFMT)
    pub(crate) convfmt: String,
    /// Subscript separator (SUBSEP)
    pub(crate) subsep: String,

    /// Current record ($0)
    pub(crate) record: String,
    /// Current fields ($1, $2, ...)
    pub(crate) fields: Vec<String>,
    /// Number of fields (NF)
    pub(crate) nf: usize,
    /// Record number (NR)
    pub(crate) nr: usize,
    /// File record number (FNR)
    pub(crate) fnr: usize,
    /// Current filename (FILENAME)
    pub(crate) filename: String,

    /// RSTART and RLENGTH from match()
    pub(crate) rstart: usize,
    pub(crate) rlength: i32,

    /// Control flow flags
    should_exit: bool,
    exit_code: i32,
    should_next: bool,
    should_nextfile: bool,

    /// Open files for output redirection
    pub(crate) output_files: HashMap<String, OutputFile>,

    /// Open files for input (getline)
    pub(crate) input_files: HashMap<String, BufReader<File>>,

    /// Open pipes for input (getline from command)
    pub(crate) pipes: HashMap<String, PipeInput>,

    /// Compiled regex cache
    pub(crate) regex_cache: HashMap<String, Regex>,

    /// Range pattern state (for /start/,/end/ patterns)
    range_states: HashMap<usize, bool>,

    /// Random number generator state
    pub(crate) rand_seed: u64,
    pub(crate) rand_state: u64,

    /// Command line arguments (ARGC, ARGV)
    pub(crate) argc: usize,
    pub(crate) argv: Vec<String>,

    /// Environment variables (ENVIRON)
    pub(crate) environ: HashMap<String, String>,
}

impl<'a> Interpreter<'a> {
    pub fn new(program: &'a Program) -> Self {
        let mut functions = HashMap::new();
        for func in &program.functions {
            functions.insert(func.name.clone(), func);
        }

        // Initialize environment variables
        let environ: HashMap<String, String> = std::env::vars().collect();

        // Initialize random seed from current time
        use std::time::{SystemTime, UNIX_EPOCH};
        let rand_seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(12345);

        Self {
            program,
            variables: HashMap::new(),
            arrays: HashMap::new(),
            functions,
            fs: " ".to_string(),
            ofs: " ".to_string(),
            rs: "\n".to_string(),
            ors: "\n".to_string(),
            ofmt: "%.6g".to_string(),
            convfmt: "%.6g".to_string(),
            subsep: "\x1c".to_string(),
            record: String::new(),
            fields: Vec::new(),
            nf: 0,
            nr: 0,
            fnr: 0,
            filename: String::new(),
            rstart: 0,
            rlength: -1,
            should_exit: false,
            exit_code: 0,
            should_next: false,
            should_nextfile: false,
            output_files: HashMap::new(),
            input_files: HashMap::new(),
            pipes: HashMap::new(),
            regex_cache: HashMap::new(),
            range_states: HashMap::new(),
            rand_seed,
            rand_state: rand_seed,
            argc: 0,
            argv: Vec::new(),
            environ,
        }
    }

    /// Set command line arguments (ARGC and ARGV)
    pub fn set_args(&mut self, args: Vec<String>) {
        self.argc = args.len();
        self.argv = args;
    }

    /// Set the field separator
    pub fn set_fs(&mut self, fs: &str) {
        self.fs = fs.to_string();
    }

    /// Set a variable before execution
    pub fn set_variable(&mut self, name: &str, value: &str) {
        self.variables.insert(name.to_string(), Value::from_string(value.to_string()));
    }

    /// Run the AWK program with given input
    pub fn run<R: BufRead, W: Write>(&mut self, inputs: Vec<R>, output: &mut W) -> Result<i32> {
        // Execute BEGIN rules
        for rule in &self.program.rules {
            if matches!(&rule.pattern, Some(Pattern::Begin)) {
                if let Some(action) = &rule.action {
                    self.execute_block(action, output)?;
                }
                if self.should_exit {
                    return Ok(self.exit_code);
                }
            }
        }

        // Process input files
        for input in inputs {
            self.fnr = 0;
            self.process_input(input, output)?;
            if self.should_exit {
                return Ok(self.exit_code);
            }
        }

        // Execute END rules
        for rule in &self.program.rules {
            if matches!(&rule.pattern, Some(Pattern::End)) {
                if let Some(action) = &rule.action {
                    self.execute_block(action, output)?;
                }
            }
        }

        Ok(self.exit_code)
    }

    fn process_input<R: BufRead, W: Write>(&mut self, mut input: R, output: &mut W) -> Result<()> {
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = input.read_line(&mut line).map_err(Error::Io)?;
            if bytes_read == 0 {
                break; // EOF
            }

            // Remove record separator
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }

            self.nr += 1;
            self.fnr += 1;
            self.set_record(&line);

            // Process main rules
            for (idx, rule) in self.program.rules.iter().enumerate() {
                if matches!(&rule.pattern, Some(Pattern::Begin) | Some(Pattern::End)) {
                    continue;
                }

                let matches = self.pattern_matches(&rule.pattern, idx)?;
                if matches {
                    if let Some(action) = &rule.action {
                        self.execute_block(action, output)?;
                    } else {
                        // Default action is to print $0
                        writeln!(output, "{}", self.record).map_err(Error::Io)?;
                    }
                }

                if self.should_next {
                    self.should_next = false;
                    break;
                }

                if self.should_nextfile || self.should_exit {
                    break;
                }
            }

            if self.should_nextfile {
                self.should_nextfile = false;
                break;
            }

            if self.should_exit {
                break;
            }
        }

        Ok(())
    }

    pub(crate) fn set_record(&mut self, record: &str) {
        self.record = record.to_string();
        self.split_fields();
    }

    fn split_fields(&mut self) {
        self.fields.clear();

        if self.record.is_empty() {
            self.nf = 0;
            return;
        }

        // Pre-estimate capacity to reduce reallocations
        let estimated_fields = self.record.len() / 8 + 1;
        self.fields.reserve(estimated_fields.min(64));

        if self.fs == " " {
            // Special case: split on runs of whitespace, trimming leading/trailing
            // Use byte-based iteration for ASCII optimization
            self.fields.extend(self.record.split_whitespace().map(String::from));
        } else if self.fs.len() == 1 {
            // Single character separator - most common case, optimize for it
            let sep = self.fs.as_bytes()[0];
            let bytes = self.record.as_bytes();
            let mut start = 0;

            for (i, &b) in bytes.iter().enumerate() {
                if b == sep {
                    self.fields.push(self.record[start..i].to_string());
                    start = i + 1;
                }
            }
            // Don't forget the last field
            self.fields.push(self.record[start..].to_string());
        } else {
            // Regex separator - cache the compiled regex
            let fs = self.fs.clone();
            let record = self.record.clone();
            if let Some(regex) = self.regex_cache.get(&fs) {
                self.fields.extend(regex.split(&record).map(String::from));
            } else if let Ok(regex) = Regex::new(&fs) {
                self.fields.extend(regex.split(&record).map(String::from));
                self.regex_cache.insert(fs, regex);
            } else {
                // If regex fails, treat as literal string
                self.fields.extend(record.split(&fs).map(String::from));
            }
        }

        self.nf = self.fields.len();
    }

    #[inline]
    pub(crate) fn get_field(&self, index: usize) -> String {
        if index == 0 {
            self.record.clone()
        } else if index <= self.fields.len() {
            self.fields[index - 1].clone()
        } else {
            String::new()
        }
    }

    /// Get field reference without cloning (for read-only access)
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn get_field_ref(&self, index: usize) -> &str {
        if index == 0 {
            &self.record
        } else if index <= self.fields.len() {
            &self.fields[index - 1]
        } else {
            ""
        }
    }

    pub(crate) fn set_field(&mut self, index: usize, value: String) {
        if index == 0 {
            self.record = value;
            self.split_fields();
        } else {
            // Extend fields if necessary
            while self.fields.len() < index {
                self.fields.push(String::new());
            }
            self.fields[index - 1] = value;
            self.nf = self.fields.len();
            // Rebuild $0
            self.record = self.fields.join(&self.ofs);
        }
    }

    fn pattern_matches(&mut self, pattern: &Option<Pattern>, rule_idx: usize) -> Result<bool> {
        match pattern {
            None => Ok(true), // No pattern means always match
            Some(Pattern::Begin) | Some(Pattern::End) => Ok(false),
            Some(Pattern::Expr(expr)) => {
                let val = self.eval_expr(expr)?;
                Ok(val.is_truthy())
            }
            Some(Pattern::Regex(regex)) => {
                let record = self.record.clone();
                let re = self.get_regex(regex)?;
                Ok(re.is_match(&record))
            }
            Some(Pattern::Range { start, end }) => {
                let active = self.range_states.get(&rule_idx).copied().unwrap_or(false);
                if !active {
                    // Check if start pattern matches
                    if self.pattern_matches(&Some(start.as_ref().clone()), rule_idx)? {
                        self.range_states.insert(rule_idx, true);
                        return Ok(true);
                    }
                    Ok(false)
                } else {
                    // Range is active, check if end pattern matches
                    if self.pattern_matches(&Some(end.as_ref().clone()), rule_idx)? {
                        self.range_states.insert(rule_idx, false);
                    }
                    Ok(true)
                }
            }
            Some(Pattern::And(left, right)) => {
                Ok(self.pattern_matches(&Some(left.as_ref().clone()), rule_idx)?
                    && self.pattern_matches(&Some(right.as_ref().clone()), rule_idx)?)
            }
            Some(Pattern::Or(left, right)) => {
                Ok(self.pattern_matches(&Some(left.as_ref().clone()), rule_idx)?
                    || self.pattern_matches(&Some(right.as_ref().clone()), rule_idx)?)
            }
            Some(Pattern::Not(inner)) => {
                Ok(!self.pattern_matches(&Some(inner.as_ref().clone()), rule_idx)?)
            }
        }
    }

    pub(crate) fn get_regex(&mut self, pattern: &str) -> Result<&Regex> {
        if !self.regex_cache.contains_key(pattern) {
            let regex = Regex::new(pattern).map_err(Error::Regex)?;
            self.regex_cache.insert(pattern.to_string(), regex);
        }
        Ok(self.regex_cache.get(pattern).unwrap())
    }

    pub(crate) fn get_variable(&self, name: &str) -> Value {
        // Check special variables first
        match name {
            "NF" => Value::Number(self.nf as f64),
            "NR" => Value::Number(self.nr as f64),
            "FNR" => Value::Number(self.fnr as f64),
            "FS" => Value::from_string(self.fs.clone()),
            "OFS" => Value::from_string(self.ofs.clone()),
            "RS" => Value::from_string(self.rs.clone()),
            "ORS" => Value::from_string(self.ors.clone()),
            "OFMT" => Value::from_string(self.ofmt.clone()),
            "CONVFMT" => Value::from_string(self.convfmt.clone()),
            "SUBSEP" => Value::from_string(self.subsep.clone()),
            "FILENAME" => Value::from_string(self.filename.clone()),
            "RSTART" => Value::Number(self.rstart as f64),
            "RLENGTH" => Value::Number(self.rlength as f64),
            "ARGC" => Value::Number(self.argc as f64),
            _ => self.variables.get(name).cloned().unwrap_or(Value::Uninitialized),
        }
    }

    /// Get an element from ARGV or ENVIRON arrays
    pub(crate) fn get_special_array(&self, array: &str, key: &str) -> Option<Value> {
        match array {
            "ARGV" => {
                key.parse::<usize>().ok()
                    .and_then(|i| self.argv.get(i))
                    .map(|s| Value::from_string(s.clone()))
            }
            "ENVIRON" => {
                self.environ.get(key).map(|s| Value::from_string(s.clone()))
            }
            _ => None,
        }
    }

    pub(crate) fn set_variable_value(&mut self, name: &str, value: Value) {
        // Handle special variables
        match name {
            "NF" => {
                let new_nf = value.to_number() as usize;
                if new_nf < self.nf {
                    self.fields.truncate(new_nf);
                } else {
                    while self.fields.len() < new_nf {
                        self.fields.push(String::new());
                    }
                }
                self.nf = new_nf;
                self.record = self.fields.join(&self.ofs);
            }
            "FS" => self.fs = value.to_string_val(),
            "OFS" => self.ofs = value.to_string_val(),
            "RS" => self.rs = value.to_string_val(),
            "ORS" => self.ors = value.to_string_val(),
            "OFMT" => self.ofmt = value.to_string_val(),
            "CONVFMT" => self.convfmt = value.to_string_val(),
            "SUBSEP" => self.subsep = value.to_string_val(),
            _ => {
                self.variables.insert(name.to_string(), value);
            }
        }
    }

    pub(crate) fn get_array_element(&self, array: &str, key: &str) -> Value {
        // Check for special arrays first
        if let Some(val) = self.get_special_array(array, key) {
            return val;
        }

        self.arrays
            .get(array)
            .and_then(|arr| arr.get(key))
            .cloned()
            .unwrap_or(Value::Uninitialized)
    }

    pub(crate) fn set_array_element(&mut self, array: &str, key: &str, value: Value) {
        self.arrays
            .entry(array.to_string())
            .or_default()
            .insert(key.to_string(), value);
    }

    pub(crate) fn array_key_exists(&self, array: &str, key: &str) -> bool {
        // Check special arrays
        match array {
            "ARGV" => {
                key.parse::<usize>().ok()
                    .map(|i| i < self.argv.len())
                    .unwrap_or(false)
            }
            "ENVIRON" => self.environ.contains_key(key),
            _ => {
                self.arrays
                    .get(array)
                    .map(|arr| arr.contains_key(key))
                    .unwrap_or(false)
            }
        }
    }

    pub(crate) fn delete_array_element(&mut self, array: &str, key: &str) {
        if let Some(arr) = self.arrays.get_mut(array) {
            arr.remove(key);
        }
    }

    pub(crate) fn make_array_key(&self, indices: &[Value]) -> String {
        indices
            .iter()
            .map(|v| v.to_string_val())
            .collect::<Vec<_>>()
            .join(&self.subsep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use std::io::Cursor;

    fn run_awk(program: &str, input: &str) -> String {
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut interpreter = Interpreter::new(&ast);
        let mut output = Vec::new();
        let inputs: Vec<std::io::BufReader<Cursor<&str>>> = if input.is_empty() {
            vec![]
        } else {
            vec![std::io::BufReader::new(Cursor::new(input))]
        };

        interpreter.run(inputs, &mut output).unwrap();
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn test_begin_print() {
        let output = run_awk(r#"BEGIN { print "hello" }"#, "");
        assert_eq!(output, "hello\n");
    }

    #[test]
    fn test_print_field() {
        let output = run_awk("{ print $1 }", "one two three");
        assert_eq!(output, "one\n");
    }

    #[test]
    fn test_print_multiple_fields() {
        let output = run_awk("{ print $1, $3 }", "one two three");
        assert_eq!(output, "one three\n");
    }

    #[test]
    fn test_arithmetic() {
        let output = run_awk("BEGIN { print 2 + 3 * 4 }", "");
        assert_eq!(output, "14\n");
    }

    #[test]
    fn test_variable() {
        let output = run_awk("BEGIN { x = 5; print x }", "");
        assert_eq!(output, "5\n");
    }

    #[test]
    fn test_if_statement() {
        let output = run_awk("BEGIN { x = 10; if (x > 5) print \"big\" }", "");
        assert_eq!(output, "big\n");
    }

    #[test]
    fn test_while_loop() {
        let output = run_awk("BEGIN { i = 1; while (i <= 3) { print i; i++ } }", "");
        assert_eq!(output, "1\n2\n3\n");
    }

    #[test]
    fn test_pattern_match() {
        let output = run_awk("/two/ { print $0 }", "one\ntwo\nthree");
        assert_eq!(output, "two\n");
    }
}
