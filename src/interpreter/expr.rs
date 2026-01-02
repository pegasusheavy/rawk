use std::io::Write;

use crate::ast::*;
use crate::error::Result;
use crate::value::{compare_values, Value};

use super::Interpreter;

impl<'a> Interpreter<'a> {
    /// Evaluate an expression (for contexts where we don't have output, like condition checking)
    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value> {
        // Use a null writer for function calls that might need output
        let mut null = std::io::sink();
        self.eval_expr_with_output(expr, &mut null)
    }

    /// Evaluate an expression with a writer for function calls that produce output
    pub fn eval_expr_with_output<W: Write>(&mut self, expr: &Expr, output: &mut W) -> Result<Value> {
        match expr {
            Expr::Number(n, _) => Ok(Value::Number(*n)),

            Expr::String(s, _) => Ok(Value::from_string(s.clone())),

            Expr::Regex(pattern, _) => {
                // Regex in expression context matches against $0
                let record = self.record.clone();
                let re = self.get_regex(pattern)?;
                Ok(Value::Number(if re.is_match(&record) { 1.0 } else { 0.0 }))
            }

            Expr::Var(name, _) => Ok(self.get_variable(name)),

            Expr::Field(expr, _) => {
                let index = self.eval_expr_with_output(expr, output)?.to_number() as usize;
                Ok(Value::from_string(self.get_field(index)))
            }

            Expr::ArrayAccess { array, indices, .. } => {
                let key_parts: Result<Vec<Value>> = indices.iter()
                    .map(|e| self.eval_expr_with_output(e, output))
                    .collect();
                let key = self.make_array_key(&key_parts?);
                Ok(self.get_array_element(array, &key))
            }

            Expr::Binary { left, op, right, .. } => {
                self.eval_binary_op_with_output(left, *op, right, output)
            }

            Expr::Unary { op, operand, .. } => {
                self.eval_unary_op_with_output(*op, operand, output)
            }

            Expr::Assign { target, op, value, .. } => {
                self.eval_assignment_with_output(target, *op, value, output)
            }

            Expr::PreIncrement(expr, _) => {
                let current = self.eval_expr_with_output(expr, output)?.to_number();
                let new_val = Value::Number(current + 1.0);
                self.assign_to_lvalue(expr, new_val.clone())?;
                Ok(new_val)
            }

            Expr::PreDecrement(expr, _) => {
                let current = self.eval_expr_with_output(expr, output)?.to_number();
                let new_val = Value::Number(current - 1.0);
                self.assign_to_lvalue(expr, new_val.clone())?;
                Ok(new_val)
            }

            Expr::PostIncrement(expr, _) => {
                let current = self.eval_expr_with_output(expr, output)?.to_number();
                let new_val = Value::Number(current + 1.0);
                self.assign_to_lvalue(expr, new_val)?;
                Ok(Value::Number(current))
            }

            Expr::PostDecrement(expr, _) => {
                let current = self.eval_expr_with_output(expr, output)?.to_number();
                let new_val = Value::Number(current - 1.0);
                self.assign_to_lvalue(expr, new_val)?;
                Ok(Value::Number(current))
            }

            Expr::Ternary { condition, then_expr, else_expr, .. } => {
                let cond = self.eval_expr_with_output(condition, output)?;
                if cond.is_truthy() {
                    self.eval_expr_with_output(then_expr, output)
                } else {
                    self.eval_expr_with_output(else_expr, output)
                }
            }

            Expr::Call { name, args, location } => {
                self.call_function(name, args, *location, output)
            }

            Expr::InArray { key, array, .. } => {
                let key_parts: Result<Vec<Value>> = key.iter()
                    .map(|e| self.eval_expr_with_output(e, output))
                    .collect();
                let key_str = self.make_array_key(&key_parts?);
                Ok(Value::Number(if self.array_key_exists(array, &key_str) { 1.0 } else { 0.0 }))
            }

            Expr::Match { expr, pattern, negated, .. } => {
                let string = self.eval_expr_with_output(expr, output)?.to_string_val();
                let pattern_str = match pattern.as_ref() {
                    Expr::Regex(p, _) => p.clone(),
                    other => self.eval_expr_with_output(other, output)?.to_string_val(),
                };
                let re = self.get_regex(&pattern_str)?;
                let matches = re.is_match(&string);
                let result = if *negated { !matches } else { matches };
                Ok(Value::Number(if result { 1.0 } else { 0.0 }))
            }

            Expr::Concat(parts, _) => {
                let mut result = String::new();
                for part in parts {
                    result.push_str(&self.eval_expr_with_output(part, output)?.to_string_val());
                }
                Ok(Value::from_string(result))
            }

            Expr::Getline { var, input, location } => {
                self.eval_getline(var.as_ref(), input.as_ref(), *location)
            }

            Expr::Group(expr, _) => self.eval_expr_with_output(expr, output),
        }
    }

    fn eval_binary_op_with_output<W: Write>(
        &mut self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
        output: &mut W,
    ) -> Result<Value> {
        // Short-circuit evaluation for logical operators
        match op {
            BinaryOp::And => {
                let l = self.eval_expr_with_output(left, output)?;
                if !l.is_truthy() {
                    return Ok(Value::Number(0.0));
                }
                let r = self.eval_expr_with_output(right, output)?;
                return Ok(Value::Number(if r.is_truthy() { 1.0 } else { 0.0 }));
            }
            BinaryOp::Or => {
                let l = self.eval_expr_with_output(left, output)?;
                if l.is_truthy() {
                    return Ok(Value::Number(1.0));
                }
                let r = self.eval_expr_with_output(right, output)?;
                return Ok(Value::Number(if r.is_truthy() { 1.0 } else { 0.0 }));
            }
            _ => {}
        }

        let l = self.eval_expr_with_output(left, output)?;
        let r = self.eval_expr_with_output(right, output)?;

        match op {
            BinaryOp::Add => Ok(Value::Number(l.to_number() + r.to_number())),
            BinaryOp::Sub => Ok(Value::Number(l.to_number() - r.to_number())),
            BinaryOp::Mul => Ok(Value::Number(l.to_number() * r.to_number())),
            BinaryOp::Div => {
                let divisor = r.to_number();
                if divisor == 0.0 {
                    Ok(Value::Number(f64::INFINITY))
                } else {
                    Ok(Value::Number(l.to_number() / divisor))
                }
            }
            BinaryOp::Mod => {
                let divisor = r.to_number();
                if divisor == 0.0 {
                    Ok(Value::Number(f64::NAN))
                } else {
                    Ok(Value::Number(l.to_number() % divisor))
                }
            }
            BinaryOp::Pow => Ok(Value::Number(l.to_number().powf(r.to_number()))),
            BinaryOp::Lt => Ok(Value::Number(if compare_values(&l, &r).is_lt() { 1.0 } else { 0.0 })),
            BinaryOp::Le => Ok(Value::Number(if compare_values(&l, &r).is_le() { 1.0 } else { 0.0 })),
            BinaryOp::Gt => Ok(Value::Number(if compare_values(&l, &r).is_gt() { 1.0 } else { 0.0 })),
            BinaryOp::Ge => Ok(Value::Number(if compare_values(&l, &r).is_ge() { 1.0 } else { 0.0 })),
            BinaryOp::Eq => Ok(Value::Number(if compare_values(&l, &r).is_eq() { 1.0 } else { 0.0 })),
            BinaryOp::Ne => Ok(Value::Number(if compare_values(&l, &r).is_ne() { 1.0 } else { 0.0 })),
            BinaryOp::Concat => {
                let mut s = l.to_string_val();
                s.push_str(&r.to_string_val());
                Ok(Value::from_string(s))
            }
            BinaryOp::And | BinaryOp::Or => unreachable!(), // Handled above
        }
    }

    fn eval_unary_op_with_output<W: Write>(
        &mut self,
        op: UnaryOp,
        operand: &Expr,
        output: &mut W,
    ) -> Result<Value> {
        let val = self.eval_expr_with_output(operand, output)?;
        match op {
            UnaryOp::Neg => Ok(Value::Number(-val.to_number())),
            UnaryOp::Pos => Ok(Value::Number(val.to_number())),
            UnaryOp::Not => Ok(Value::Number(if val.is_truthy() { 0.0 } else { 1.0 })),
        }
    }

    fn eval_assignment_with_output<W: Write>(
        &mut self,
        target: &Expr,
        op: AssignOp,
        value: &Expr,
        output: &mut W,
    ) -> Result<Value> {
        let new_value = match op {
            AssignOp::Assign => self.eval_expr_with_output(value, output)?,
            _ => {
                let current = self.eval_expr_with_output(target, output)?;
                let rhs = self.eval_expr_with_output(value, output)?;
                match op {
                    AssignOp::AddAssign => Value::Number(current.to_number() + rhs.to_number()),
                    AssignOp::SubAssign => Value::Number(current.to_number() - rhs.to_number()),
                    AssignOp::MulAssign => Value::Number(current.to_number() * rhs.to_number()),
                    AssignOp::DivAssign => Value::Number(current.to_number() / rhs.to_number()),
                    AssignOp::ModAssign => Value::Number(current.to_number() % rhs.to_number()),
                    AssignOp::PowAssign => Value::Number(current.to_number().powf(rhs.to_number())),
                    AssignOp::Assign => unreachable!(),
                }
            }
        };

        self.assign_to_lvalue(target, new_value.clone())?;
        Ok(new_value)
    }

    /// Evaluate getline expression
    pub(crate) fn eval_getline(
        &mut self,
        var: Option<&String>,
        input: Option<&GetlineInput>,
        _location: crate::error::SourceLocation,
    ) -> Result<Value> {
        use std::io::BufRead;

        match input {
            None => {
                // getline with no input source - read from current input
                // This is handled by the main loop, so we return 0 (EOF) here
                Ok(Value::Number(0.0))
            }
            Some(GetlineInput::File(file_expr)) => {
                let filename = self.eval_expr(file_expr)?.to_string_val();

                // Get or open the file
                if !self.input_files.contains_key(&filename) {
                    match std::fs::File::open(&filename) {
                        Ok(file) => {
                            self.input_files.insert(filename.clone(), std::io::BufReader::new(file));
                        }
                        Err(_) => return Ok(Value::Number(-1.0)), // Error
                    }
                }

                let reader = self.input_files.get_mut(&filename).unwrap();
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => Ok(Value::Number(0.0)), // EOF
                    Ok(_) => {
                        // Remove trailing newline
                        if line.ends_with('\n') {
                            line.pop();
                            if line.ends_with('\r') {
                                line.pop();
                            }
                        }

                        if let Some(var_name) = var {
                            self.set_variable_value(var_name, Value::from_string(line));
                        } else {
                            self.set_record(&line);
                        }
                        Ok(Value::Number(1.0)) // Success
                    }
                    Err(_) => Ok(Value::Number(-1.0)), // Error
                }
            }
            Some(GetlineInput::Pipe(cmd_expr)) => {
                let cmd = self.eval_expr(cmd_expr)?.to_string_val();

                // Get or open the pipe
                if !self.pipes.contains_key(&cmd) {
                    match std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&cmd)
                        .stdout(std::process::Stdio::piped())
                        .spawn()
                    {
                        Ok(mut child) => {
                            let stdout = child.stdout.take().unwrap();
                            self.pipes.insert(cmd.clone(), super::PipeInput {
                                child,
                                reader: std::io::BufReader::new(stdout),
                            });
                        }
                        Err(_) => return Ok(Value::Number(-1.0)), // Error
                    }
                }

                let pipe = self.pipes.get_mut(&cmd).unwrap();
                let mut line = String::new();
                match pipe.reader.read_line(&mut line) {
                    Ok(0) => Ok(Value::Number(0.0)), // EOF
                    Ok(_) => {
                        // Remove trailing newline
                        if line.ends_with('\n') {
                            line.pop();
                            if line.ends_with('\r') {
                                line.pop();
                            }
                        }

                        if let Some(var_name) = var {
                            self.set_variable_value(var_name, Value::from_string(line));
                        } else {
                            self.set_record(&line);
                        }
                        Ok(Value::Number(1.0)) // Success
                    }
                    Err(_) => Ok(Value::Number(-1.0)), // Error
                }
            }
        }
    }

    pub fn assign_to_lvalue(&mut self, target: &Expr, value: Value) -> Result<()> {
        match target {
            Expr::Var(name, _) => {
                self.set_variable_value(name, value);
            }
            Expr::Field(expr, _) => {
                let index = self.eval_expr(expr)?.to_number() as usize;
                self.set_field(index, value.to_string_val());
            }
            Expr::ArrayAccess { array, indices, .. } => {
                let key_parts: Result<Vec<Value>> = indices.iter().map(|e| self.eval_expr(e)).collect();
                let key = self.make_array_key(&key_parts?);
                self.set_array_element(array, &key, value);
            }
            _ => {
                // Invalid lvalue, but we'll just ignore for now
            }
        }
        Ok(())
    }
}
