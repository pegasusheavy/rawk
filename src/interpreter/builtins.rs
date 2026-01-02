use std::io::Write;

use crate::ast::Expr;
use crate::error::{Error, Result, SourceLocation};
use crate::value::Value;

use super::Interpreter;

impl<'a> Interpreter<'a> {
    /// Call a function with special handling for builtins that need AST access
    pub fn call_function<W: Write>(
        &mut self,
        name: &str,
        args: &[Expr],
        location: SourceLocation,
        output: &mut W,
    ) -> Result<Value> {
        // Check for built-in functions that need special argument handling
        match name {
            "sub" | "gsub" => return self.call_regex_sub(name, args, location),
            "match" => return self.call_match(args, location),
            "split" => return self.call_split(args, location),
            "getline" => return self.call_getline(args, location),
            "close" => return self.call_close(args, location),
            "fflush" => return self.call_fflush(args, location, output),
            _ => {}
        }

        // Evaluate all arguments for other functions
        let arg_values: Result<Vec<Value>> = args.iter().map(|e| self.eval_expr(e)).collect();
        let arg_values = arg_values?;

        // Check for other built-in functions
        if let Some(result) = self.call_builtin(name, &arg_values)? {
            return Ok(result);
        }

        // Check for user-defined functions
        if let Some(func) = self.functions.get(name).cloned() {
            return self.call_user_function(func, arg_values, output);
        }

        Err(Error::runtime_at(
            format!("undefined function: {}", name),
            location.line,
            location.column,
        ))
    }

    /// Extract regex pattern from an expression (handles both regex literals and strings)
    fn extract_pattern(&mut self, expr: &Expr) -> Result<String> {
        match expr {
            Expr::Regex(pattern, _) => Ok(pattern.clone()),
            other => Ok(self.eval_expr(other)?.to_string_val()),
        }
    }

    /// Call sub or gsub with proper regex and target handling
    fn call_regex_sub(&mut self, name: &str, args: &[Expr], location: SourceLocation) -> Result<Value> {
        let global = name == "gsub";

        let pattern = args.first()
            .map(|e| self.extract_pattern(e))
            .transpose()?
            .unwrap_or_default();

        let replacement = args.get(1)
            .map(|e| self.eval_expr(e))
            .transpose()?
            .map(|v| v.to_string_val())
            .unwrap_or_default();

        // Get the target (third argument or $0)
        let (target_value, target_expr) = if let Some(target_arg) = args.get(2) {
            (self.eval_expr(target_arg)?.to_string_val(), Some(target_arg))
        } else {
            (self.record.clone(), None)
        };

        let re = regex::Regex::new(&pattern).map_err(|e| {
            Error::runtime_at(format!("invalid regex: {}", e), location.line, location.column)
        })?;

        let (new_str, count) = regex_sub_helper(&re, &replacement, &target_value, global);

        // Assign the result back to the target
        if let Some(target_arg) = target_expr {
            self.assign_to_lvalue(target_arg, Value::from_string(new_str))?;
        } else {
            self.set_record(&new_str);
        }

        Ok(Value::Number(count as f64))
    }

    /// Call match with proper regex handling
    fn call_match(&mut self, args: &[Expr], location: SourceLocation) -> Result<Value> {
        let s = args.first()
            .map(|e| self.eval_expr(e))
            .transpose()?
            .map(|v| v.to_string_val())
            .unwrap_or_default();

        let pattern = args.get(1)
            .map(|e| self.extract_pattern(e))
            .transpose()?
            .unwrap_or_default();

        let re = regex::Regex::new(&pattern).map_err(|e| {
            Error::runtime_at(format!("invalid regex: {}", e), location.line, location.column)
        })?;

        if let Some(m) = re.find(&s) {
            self.rstart = m.start() + 1;
            self.rlength = m.len() as i32;
            Ok(Value::Number(self.rstart as f64))
        } else {
            self.rstart = 0;
            self.rlength = -1;
            Ok(Value::Number(0.0))
        }
    }

    /// Call split with proper array name handling
    fn call_split(&mut self, args: &[Expr], location: SourceLocation) -> Result<Value> {
        let s = args.first()
            .map(|e| self.eval_expr(e))
            .transpose()?
            .map(|v| v.to_string_val())
            .unwrap_or_default();

        // Get array name from second argument (must be a variable name)
        let array_name = match args.get(1) {
            Some(Expr::Var(name, _)) => name.clone(),
            Some(Expr::ArrayAccess { array, .. }) => array.clone(),
            Some(_) => {
                return Err(Error::runtime_at(
                    "split: second argument must be an array",
                    location.line,
                    location.column,
                ));
            }
            None => {
                return Err(Error::runtime_at(
                    "split: missing array argument",
                    location.line,
                    location.column,
                ));
            }
        };

        // Get separator (third argument or default FS)
        let sep = if let Some(sep_expr) = args.get(2) {
            self.extract_pattern(sep_expr)?
        } else {
            self.fs.clone()
        };

        // Clear the array
        self.arrays.remove(&array_name);

        // Split and populate array
        let parts: Vec<&str> = if sep == " " {
            s.split_whitespace().collect()
        } else if sep.len() == 1 {
            s.split(&sep).collect()
        } else {
            // Use regex split for multi-char separators
            let re = regex::Regex::new(&sep).map_err(|e| {
                Error::runtime_at(format!("invalid regex: {}", e), location.line, location.column)
            })?;
            re.split(&s).collect()
        };

        for (i, part) in parts.iter().enumerate() {
            let key = (i + 1).to_string();
            self.set_array_element(&array_name, &key, Value::from_string(part.to_string()));
        }

        Ok(Value::Number(parts.len() as f64))
    }

    /// Call getline with file/pipe/variable handling
    fn call_getline(&mut self, args: &[Expr], location: SourceLocation) -> Result<Value> {
        // getline returns: 1 (success), 0 (EOF), -1 (error)
        // For now, just return 0 (EOF) for unsupported cases
        // TODO: Implement proper getline with file/pipe support
        let _ = args;
        let _ = location;
        Ok(Value::Number(0.0))
    }

    /// Call close to close a file or pipe
    fn call_close(&mut self, args: &[Expr], location: SourceLocation) -> Result<Value> {
        let filename = args.first()
            .map(|e| self.eval_expr(e))
            .transpose()?
            .map(|v| v.to_string_val())
            .unwrap_or_default();

        // Remove from output files if it exists
        if self.output_files.remove(&filename).is_some() {
            Ok(Value::Number(0.0))  // Success
        } else if self.input_files.remove(&filename).is_some() {
            Ok(Value::Number(0.0))  // Success
        } else if self.pipes.remove(&filename).is_some() {
            Ok(Value::Number(0.0))  // Success
        } else {
            let _ = location;
            Ok(Value::Number(-1.0)) // Not found
        }
    }

    /// Call fflush to flush output
    fn call_fflush<W: Write>(&mut self, args: &[Expr], _location: SourceLocation, output: &mut W) -> Result<Value> {
        if args.is_empty() {
            // Flush all output
            output.flush().map_err(Error::Io)?;
            for file in self.output_files.values_mut() {
                let _ = file.flush();
            }
            Ok(Value::Number(0.0))
        } else {
            let filename = self.eval_expr(&args[0])?.to_string_val();
            if filename.is_empty() {
                output.flush().map_err(Error::Io)?;
                Ok(Value::Number(0.0))
            } else if let Some(file) = self.output_files.get_mut(&filename) {
                file.flush().map_err(Error::Io)?;
                Ok(Value::Number(0.0))
            } else {
                Ok(Value::Number(-1.0))
            }
        }
    }

    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Option<Value>> {
        match name {
            // String functions
            "length" => {
                let s = args.first().map(|v| v.to_string_val()).unwrap_or_else(|| self.record.clone());
                Ok(Some(Value::Number(s.len() as f64)))
            }

            "substr" => {
                let s = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                let start = args.get(1).map(|v| v.to_number() as usize).unwrap_or(1);
                let len = args.get(2).map(|v| v.to_number() as usize);

                // AWK uses 1-based indexing
                let start = start.saturating_sub(1).min(s.len());
                let result = if let Some(len) = len {
                    s.chars().skip(start).take(len).collect()
                } else {
                    s.chars().skip(start).collect()
                };
                Ok(Some(Value::from_string(result)))
            }

            "index" => {
                let s = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                let target = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
                let pos = s.find(&target).map(|i| i + 1).unwrap_or(0);
                Ok(Some(Value::Number(pos as f64)))
            }

            "sprintf" => {
                let format = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                let rest = if args.len() > 1 { &args[1..] } else { &[] };
                let result = self.format_printf(&format, rest);
                Ok(Some(Value::from_string(result)))
            }

            "tolower" => {
                let s = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                Ok(Some(Value::from_string(s.to_lowercase())))
            }

            "toupper" => {
                let s = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                Ok(Some(Value::from_string(s.to_uppercase())))
            }

            // Math functions
            "sin" => {
                let n = args.first().map(|v| v.to_number()).unwrap_or(0.0);
                Ok(Some(Value::Number(n.sin())))
            }

            "cos" => {
                let n = args.first().map(|v| v.to_number()).unwrap_or(0.0);
                Ok(Some(Value::Number(n.cos())))
            }

            "atan2" => {
                let y = args.first().map(|v| v.to_number()).unwrap_or(0.0);
                let x = args.get(1).map(|v| v.to_number()).unwrap_or(0.0);
                Ok(Some(Value::Number(y.atan2(x))))
            }

            "exp" => {
                let n = args.first().map(|v| v.to_number()).unwrap_or(0.0);
                Ok(Some(Value::Number(n.exp())))
            }

            "log" => {
                let n = args.first().map(|v| v.to_number()).unwrap_or(0.0);
                Ok(Some(Value::Number(n.ln())))
            }

            "sqrt" => {
                let n = args.first().map(|v| v.to_number()).unwrap_or(0.0);
                Ok(Some(Value::Number(n.sqrt())))
            }

            "int" => {
                let n = args.first().map(|v| v.to_number()).unwrap_or(0.0);
                Ok(Some(Value::Number(n.trunc())))
            }

            "rand" => {
                // Use the internal RNG state
                let random = self.next_random();
                Ok(Some(Value::Number(random)))
            }

            "srand" => {
                let old_seed = self.rand_seed;
                if let Some(seed) = args.first() {
                    self.rand_seed = seed.to_number() as u64;
                } else {
                    use std::time::{SystemTime, UNIX_EPOCH};
                    self.rand_seed = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                }
                self.rand_state = self.rand_seed;
                Ok(Some(Value::Number(old_seed as f64)))
            }

            // System functions
            "system" => {
                let cmd = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .status()
                    .map(|s| s.code().unwrap_or(-1))
                    .unwrap_or(-1);
                Ok(Some(Value::Number(status as f64)))
            }

            _ => Ok(None), // Not a built-in
        }
    }

    fn call_user_function<W: Write>(
        &mut self,
        func: &crate::ast::FunctionDef,
        args: Vec<Value>,
        output: &mut W,
    ) -> Result<Value> {
        // Save current variables for local scope
        let saved_vars: std::collections::HashMap<String, Value> = func.params.iter()
            .filter_map(|name| self.variables.get(name).map(|v| (name.clone(), v.clone())))
            .collect();

        // Set parameters
        for (i, param) in func.params.iter().enumerate() {
            let value = args.get(i).cloned().unwrap_or(Value::Uninitialized);
            self.set_variable_value(param, value);
        }

        // Execute function body, passing the actual output
        let result = match self.execute_block(&func.body, output)? {
            super::stmt::StmtResult::Return(v) => v,
            _ => Value::Uninitialized,
        };

        // Restore saved variables and remove parameters that weren't saved
        for param in &func.params {
            if let Some(value) = saved_vars.get(param) {
                self.set_variable_value(param, value.clone());
            } else {
                self.variables.remove(param);
            }
        }

        Ok(result)
    }

    /// Generate a random number between 0 and 1 using xorshift64
    fn next_random(&mut self) -> f64 {
        let mut x = self.rand_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rand_state = x;
        (x as f64) / (u64::MAX as f64)
    }
}

fn regex_sub_helper(re: &regex::Regex, replacement: &str, target: &str, global: bool) -> (String, usize) {
    // Handle & in replacement (matched text)
    let mut count = 0;

    if global {
        let result = re.replace_all(target, |caps: &regex::Captures| {
            count += 1;
            replacement.replace("&", caps.get(0).map(|m| m.as_str()).unwrap_or(""))
        });
        (result.to_string(), count)
    } else {
        let result = re.replace(target, |caps: &regex::Captures| {
            count += 1;
            replacement.replace("&", caps.get(0).map(|m| m.as_str()).unwrap_or(""))
        });
        (result.to_string(), count)
    }
}
