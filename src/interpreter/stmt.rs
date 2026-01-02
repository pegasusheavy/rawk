use std::io::Write;
use std::fs::{File, OpenOptions};
use std::process::{Command, Stdio};

use crate::ast::*;
use crate::error::{Error, Result};
use crate::value::Value;

use super::{Interpreter, OutputFile};

/// Result of executing a statement
pub enum StmtResult {
    Normal,
    Break,
    Continue,
    Return(Value),
}

impl<'a> Interpreter<'a> {
    pub fn execute_block<W: Write>(&mut self, block: &Block, output: &mut W) -> Result<StmtResult> {
        for stmt in &block.statements {
            let result = self.execute_stmt(stmt, output)?;
            match result {
                StmtResult::Normal => continue,
                other => return Ok(other),
            }
        }
        Ok(StmtResult::Normal)
    }

    pub fn execute_stmt<W: Write>(&mut self, stmt: &Stmt, output: &mut W) -> Result<StmtResult> {
        match stmt {
            Stmt::Empty => Ok(StmtResult::Normal),

            Stmt::Expr(expr) => {
                self.eval_expr_with_output(expr, output)?;
                Ok(StmtResult::Normal)
            }

            Stmt::Print { args, output: redirect, .. } => {
                self.execute_print(args, redirect, output)?;
                Ok(StmtResult::Normal)
            }

            Stmt::Printf { format, args, output: redirect, .. } => {
                self.execute_printf(format, args, redirect, output)?;
                Ok(StmtResult::Normal)
            }

            Stmt::If { condition, then_branch, else_branch, .. } => {
                let cond = self.eval_expr_with_output(condition, output)?;
                if cond.is_truthy() {
                    self.execute_stmt(then_branch, output)
                } else if let Some(else_stmt) = else_branch {
                    self.execute_stmt(else_stmt, output)
                } else {
                    Ok(StmtResult::Normal)
                }
            }

            Stmt::While { condition, body, .. } => {
                loop {
                    let cond = self.eval_expr_with_output(condition, output)?;
                    if !cond.is_truthy() {
                        break;
                    }
                    match self.execute_stmt(body, output)? {
                        StmtResult::Normal | StmtResult::Continue => continue,
                        StmtResult::Break => break,
                        StmtResult::Return(v) => return Ok(StmtResult::Return(v)),
                    }
                }
                Ok(StmtResult::Normal)
            }

            Stmt::DoWhile { body, condition, .. } => {
                loop {
                    match self.execute_stmt(body, output)? {
                        StmtResult::Normal | StmtResult::Continue => {}
                        StmtResult::Break => break,
                        StmtResult::Return(v) => return Ok(StmtResult::Return(v)),
                    }
                    let cond = self.eval_expr_with_output(condition, output)?;
                    if !cond.is_truthy() {
                        break;
                    }
                }
                Ok(StmtResult::Normal)
            }

            Stmt::For { init, condition, update, body, .. } => {
                // Execute init
                if let Some(init_stmt) = init {
                    self.execute_stmt(init_stmt, output)?;
                }

                loop {
                    // Check condition
                    if let Some(cond_expr) = condition {
                        let cond = self.eval_expr_with_output(cond_expr, output)?;
                        if !cond.is_truthy() {
                            break;
                        }
                    }

                    // Execute body
                    match self.execute_stmt(body, output)? {
                        StmtResult::Normal | StmtResult::Continue => {}
                        StmtResult::Break => break,
                        StmtResult::Return(v) => return Ok(StmtResult::Return(v)),
                    }

                    // Execute update
                    if let Some(update_expr) = update {
                        self.eval_expr_with_output(update_expr, output)?;
                    }
                }
                Ok(StmtResult::Normal)
            }

            Stmt::ForIn { var, array, body, .. } => {
                // Get keys from array
                let keys: Vec<String> = self.arrays
                    .get(array)
                    .map(|arr| arr.keys().cloned().collect())
                    .unwrap_or_default();

                for key in keys {
                    self.set_variable_value(var, Value::from_string(key));
                    match self.execute_stmt(body, output)? {
                        StmtResult::Normal | StmtResult::Continue => continue,
                        StmtResult::Break => break,
                        StmtResult::Return(v) => return Ok(StmtResult::Return(v)),
                    }
                }
                Ok(StmtResult::Normal)
            }

            Stmt::Block(block) => self.execute_block(block, output),

            Stmt::Break { .. } => Ok(StmtResult::Break),

            Stmt::Continue { .. } => Ok(StmtResult::Continue),

            Stmt::Next { .. } => {
                self.should_next = true;
                Ok(StmtResult::Normal)
            }

            Stmt::Nextfile { .. } => {
                self.should_nextfile = true;
                Ok(StmtResult::Normal)
            }

            Stmt::Exit { code, .. } => {
                self.exit_code = code.as_ref()
                    .map(|e| self.eval_expr_with_output(e, output).map(|v| v.to_number() as i32))
                    .transpose()?
                    .unwrap_or(0);
                self.should_exit = true;
                Ok(StmtResult::Normal)
            }

            Stmt::Return { value, .. } => {
                let val = value.as_ref()
                    .map(|e| self.eval_expr_with_output(e, output))
                    .transpose()?
                    .unwrap_or(Value::Uninitialized);
                Ok(StmtResult::Return(val))
            }

            Stmt::Delete { array, index, .. } => {
                if index.is_empty() {
                    // delete array (entire array)
                    self.arrays.remove(array);
                } else {
                    let key_parts: Result<Vec<Value>> = index.iter()
                        .map(|e| self.eval_expr_with_output(e, output))
                        .collect();
                    let key = self.make_array_key(&key_parts?);
                    self.delete_array_element(array, &key);
                }
                Ok(StmtResult::Normal)
            }

            Stmt::Getline { var, input, location } => {
                // Getline as a statement
                let _result = self.eval_getline(var.as_ref(), input.as_ref(), *location)?;
                Ok(StmtResult::Normal)
            }
        }
    }

    fn execute_print<W: Write>(
        &mut self,
        args: &[Expr],
        redirect: &Option<OutputRedirect>,
        default_output: &mut W,
    ) -> Result<()> {
        let values: Result<Vec<String>> = args.iter()
            .map(|e| self.eval_expr_with_output(e, default_output).map(|v| v.to_string_val()))
            .collect();
        let values = values?;

        let line = if values.is_empty() {
            // print without args prints $0
            self.record.clone()
        } else {
            values.join(&self.ofs)
        };

        // Handle output redirection
        match redirect {
            None => {
                writeln!(default_output, "{}", line).map_err(Error::Io)?;
            }
            Some(OutputRedirect::Truncate(target_expr)) => {
                let filename = self.eval_expr_with_output(target_expr, default_output)?.to_string_val();
                let file = self.get_or_open_file(&filename, false)?;
                writeln!(file, "{}", line).map_err(Error::Io)?;
            }
            Some(OutputRedirect::Append(target_expr)) => {
                let filename = self.eval_expr_with_output(target_expr, default_output)?.to_string_val();
                let file = self.get_or_open_file(&filename, true)?;
                writeln!(file, "{}", line).map_err(Error::Io)?;
            }
            Some(OutputRedirect::Pipe(cmd_expr)) => {
                let cmd = self.eval_expr_with_output(cmd_expr, default_output)?.to_string_val();
                let pipe = self.get_or_open_pipe(&cmd)?;
                writeln!(pipe, "{}", line).map_err(Error::Io)?;
            }
        }

        Ok(())
    }

    fn execute_printf<W: Write>(
        &mut self,
        format_expr: &Expr,
        args: &[Expr],
        redirect: &Option<OutputRedirect>,
        default_output: &mut W,
    ) -> Result<()> {
        let format = self.eval_expr_with_output(format_expr, default_output)?.to_string_val();
        let values: Result<Vec<Value>> = args.iter()
            .map(|e| self.eval_expr_with_output(e, default_output))
            .collect();
        let values = values?;

        let formatted = self.format_printf(&format, &values);

        // Handle output redirection
        match redirect {
            None => {
                write!(default_output, "{}", formatted).map_err(Error::Io)?;
            }
            Some(OutputRedirect::Truncate(target_expr)) => {
                let filename = self.eval_expr_with_output(target_expr, default_output)?.to_string_val();
                let file = self.get_or_open_file(&filename, false)?;
                write!(file, "{}", formatted).map_err(Error::Io)?;
            }
            Some(OutputRedirect::Append(target_expr)) => {
                let filename = self.eval_expr_with_output(target_expr, default_output)?.to_string_val();
                let file = self.get_or_open_file(&filename, true)?;
                write!(file, "{}", formatted).map_err(Error::Io)?;
            }
            Some(OutputRedirect::Pipe(cmd_expr)) => {
                let cmd = self.eval_expr_with_output(cmd_expr, default_output)?.to_string_val();
                let pipe = self.get_or_open_pipe(&cmd)?;
                write!(pipe, "{}", formatted).map_err(Error::Io)?;
            }
        }

        Ok(())
    }

    /// Get or open a file for output redirection
    fn get_or_open_file(&mut self, filename: &str, append: bool) -> Result<&mut OutputFile> {
        if !self.output_files.contains_key(filename) {
            let file = if append {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(filename)
                    .map_err(Error::Io)?
            } else {
                File::create(filename).map_err(Error::Io)?
            };
            self.output_files.insert(filename.to_string(), OutputFile::File(file));
        }
        Ok(self.output_files.get_mut(filename).unwrap())
    }

    /// Get or open a pipe for output redirection
    fn get_or_open_pipe(&mut self, cmd: &str) -> Result<&mut OutputFile> {
        if !self.output_files.contains_key(cmd) {
            let child = Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .stdin(Stdio::piped())
                .spawn()
                .map_err(Error::Io)?;

            let stdin = child.stdin.unwrap();
            self.output_files.insert(cmd.to_string(), OutputFile::Pipe(stdin));
        }
        Ok(self.output_files.get_mut(cmd).unwrap())
    }

    pub(crate) fn format_printf(&self, format: &str, args: &[Value]) -> String {
        let mut result = String::new();
        let mut chars = format.chars().peekable();
        let mut arg_idx = 0;

        while let Some(ch) = chars.next() {
            if ch != '%' {
                result.push(ch);
                continue;
            }

            // Check for %%
            if chars.peek() == Some(&'%') {
                chars.next();
                result.push('%');
                continue;
            }

            // Parse format specifier
            let mut width = String::new();
            let mut precision = String::new();
            let mut flags = String::new();

            // Flags
            while let Some(&c) = chars.peek() {
                if c == '-' || c == '+' || c == ' ' || c == '#' || c == '0' {
                    flags.push(c);
                    chars.next();
                } else {
                    break;
                }
            }

            // Width
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() {
                    width.push(c);
                    chars.next();
                } else {
                    break;
                }
            }

            // Precision
            if chars.peek() == Some(&'.') {
                chars.next();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() {
                        precision.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
            }

            // Conversion specifier
            let spec = chars.next().unwrap_or('s');
            let arg = args.get(arg_idx).cloned().unwrap_or(Value::Uninitialized);
            arg_idx += 1;

            let width_num: Option<usize> = width.parse().ok();
            let precision_num: Option<usize> = precision.parse().ok();
            let left_align = flags.contains('-');

            let formatted = match spec {
                's' => {
                    let s = arg.to_string_val();
                    let s = if let Some(p) = precision_num {
                        s.chars().take(p).collect()
                    } else {
                        s
                    };
                    if let Some(w) = width_num {
                        if left_align {
                            format!("{:<width$}", s, width = w)
                        } else {
                            format!("{:>width$}", s, width = w)
                        }
                    } else {
                        s
                    }
                }
                'd' | 'i' => {
                    let n = arg.to_number() as i64;
                    if let Some(w) = width_num {
                        if flags.contains('0') && !left_align {
                            format!("{:0>width$}", n, width = w)
                        } else if left_align {
                            format!("{:<width$}", n, width = w)
                        } else {
                            format!("{:>width$}", n, width = w)
                        }
                    } else {
                        format!("{}", n)
                    }
                }
                'f' | 'F' => {
                    let n = arg.to_number();
                    let p = precision_num.unwrap_or(6);
                    if let Some(w) = width_num {
                        if left_align {
                            format!("{:<width$.prec$}", n, width = w, prec = p)
                        } else {
                            format!("{:>width$.prec$}", n, width = w, prec = p)
                        }
                    } else {
                        format!("{:.prec$}", n, prec = p)
                    }
                }
                'e' | 'E' => {
                    let n = arg.to_number();
                    let p = precision_num.unwrap_or(6);
                    format!("{:.prec$e}", n, prec = p)
                }
                'g' | 'G' => {
                    let n = arg.to_number();
                    let p = precision_num.unwrap_or(6);
                    // Simplified %g implementation
                    if n.abs() >= 1e-4 && n.abs() < 10f64.powi(p as i32) {
                        format!("{:.prec$}", n, prec = p)
                    } else {
                        format!("{:.prec$e}", n, prec = p)
                    }
                }
                'o' => format!("{:o}", arg.to_number() as u64),
                'x' => format!("{:x}", arg.to_number() as u64),
                'X' => format!("{:X}", arg.to_number() as u64),
                'c' => {
                    let n = arg.to_number() as u32;
                    char::from_u32(n).map(|c| c.to_string()).unwrap_or_default()
                }
                _ => format!("%{}", spec),
            };

            result.push_str(&formatted);
        }

        result
    }
}
