use crate::ast::*;
use crate::error::{Error, Result, SourceLocation};
use crate::lexer::{Token, TokenKind};

/// AWK parser using recursive descent
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    /// Parse a complete AWK program
    pub fn parse(&mut self) -> Result<Program> {
        let mut program = Program::new();

        self.skip_newlines();

        while !self.is_at_end() {
            // Check for function definition
            if self.check(&TokenKind::Function) {
                program.functions.push(self.parse_function()?);
            } else {
                program.rules.push(self.parse_rule()?);
            }
            self.skip_newlines();
        }

        Ok(program)
    }

    /// Parse a function definition
    fn parse_function(&mut self) -> Result<FunctionDef> {
        let location = self.current_location();
        self.expect(&TokenKind::Function)?;

        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LeftParen)?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            params.push(self.expect_identifier()?);
            while self.match_token(&TokenKind::Comma) {
                params.push(self.expect_identifier()?);
            }
        }
        self.expect(&TokenKind::RightParen)?;
        self.skip_newlines();

        let body = self.parse_block()?;

        Ok(FunctionDef {
            name,
            params,
            body,
            location,
        })
    }

    /// Parse a pattern-action rule
    fn parse_rule(&mut self) -> Result<Rule> {
        let location = self.current_location();

        // Check for BEGIN/END
        if self.check(&TokenKind::Begin) {
            self.advance();
            self.skip_newlines();
            let action = Some(self.parse_block()?);
            return Ok(Rule {
                pattern: Some(Pattern::Begin),
                action,
                location,
            });
        }

        if self.check(&TokenKind::End) {
            self.advance();
            self.skip_newlines();
            let action = Some(self.parse_block()?);
            return Ok(Rule {
                pattern: Some(Pattern::End),
                action,
                location,
            });
        }

        // Check for action-only rule (just a block)
        if self.check(&TokenKind::LeftBrace) {
            let action = Some(self.parse_block()?);
            return Ok(Rule {
                pattern: None,
                action,
                location,
            });
        }

        // Parse pattern
        let pattern = Some(self.parse_pattern()?);
        self.skip_newlines();

        // Optional action
        let action = if self.check(&TokenKind::LeftBrace) {
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Rule {
            pattern,
            action,
            location,
        })
    }

    /// Parse a pattern (expression or regex)
    fn parse_pattern(&mut self) -> Result<Pattern> {
        // Check for regex pattern
        if let Some(TokenKind::Regex(pattern)) = self.peek_kind() {
            let pattern = pattern.clone();
            self.advance();

            // Check for range pattern
            if self.match_token(&TokenKind::Comma) {
                self.skip_newlines();
                let end = self.parse_pattern()?;
                return Ok(Pattern::Range {
                    start: Box::new(Pattern::Regex(pattern)),
                    end: Box::new(end),
                });
            }

            return Ok(Pattern::Regex(pattern));
        }

        // Parse as expression pattern
        let expr = self.parse_expression()?;

        // Check for range pattern
        if self.match_token(&TokenKind::Comma) {
            self.skip_newlines();
            let end = self.parse_pattern()?;
            return Ok(Pattern::Range {
                start: Box::new(Pattern::Expr(expr)),
                end: Box::new(end),
            });
        }

        Ok(Pattern::Expr(expr))
    }

    /// Parse a block { ... }
    fn parse_block(&mut self) -> Result<Block> {
        let location = self.current_location();
        self.expect(&TokenKind::LeftBrace)?;
        self.skip_newlines();

        let mut statements = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
            self.skip_terminators();
        }

        self.expect(&TokenKind::RightBrace)?;

        Ok(Block::new(statements, location))
    }

    /// Parse a single statement
    fn parse_statement(&mut self) -> Result<Stmt> {
        self.skip_newlines();

        let location = self.current_location();

        // Empty statement
        if self.check(&TokenKind::Semicolon) {
            self.advance();
            return Ok(Stmt::Empty);
        }

        // Block
        if self.check(&TokenKind::LeftBrace) {
            return Ok(Stmt::Block(self.parse_block()?));
        }

        // If statement
        if self.match_token(&TokenKind::If) {
            return self.parse_if_statement(location);
        }

        // While statement
        if self.match_token(&TokenKind::While) {
            return self.parse_while_statement(location);
        }

        // For statement
        if self.match_token(&TokenKind::For) {
            return self.parse_for_statement(location);
        }

        // Do-while statement
        if self.match_token(&TokenKind::Do) {
            return self.parse_do_while_statement(location);
        }

        // Break
        if self.match_token(&TokenKind::Break) {
            return Ok(Stmt::Break { location });
        }

        // Continue
        if self.match_token(&TokenKind::Continue) {
            return Ok(Stmt::Continue { location });
        }

        // Next
        if self.match_token(&TokenKind::Next) {
            return Ok(Stmt::Next { location });
        }

        // Nextfile
        if self.match_token(&TokenKind::Nextfile) {
            return Ok(Stmt::Nextfile { location });
        }

        // Exit
        if self.match_token(&TokenKind::Exit) {
            let code = if self.can_start_expression() {
                Some(self.parse_expression()?)
            } else {
                None
            };
            return Ok(Stmt::Exit { code, location });
        }

        // Return
        if self.match_token(&TokenKind::Return) {
            let value = if self.can_start_expression() {
                Some(self.parse_expression()?)
            } else {
                None
            };
            return Ok(Stmt::Return { value, location });
        }

        // Delete
        if self.match_token(&TokenKind::Delete) {
            let name = self.expect_identifier()?;

            // Check if there's an index (delete array[i]) or not (delete array)
            let indices = if self.match_token(&TokenKind::LeftBracket) {
                let mut indices = vec![self.parse_expression()?];
                while self.match_token(&TokenKind::Comma) {
                    indices.push(self.parse_expression()?);
                }
                self.expect(&TokenKind::RightBracket)?;
                indices
            } else {
                // delete array (entire array)
                Vec::new()
            };

            return Ok(Stmt::Delete {
                array: name,
                index: indices,
                location,
            });
        }

        // Print statement
        if self.match_token(&TokenKind::Print) {
            return self.parse_print_statement(location);
        }

        // Printf statement
        if self.match_token(&TokenKind::Printf) {
            return self.parse_printf_statement(location);
        }

        // Expression statement
        let expr = self.parse_expression()?;
        Ok(Stmt::Expr(expr))
    }

    fn parse_if_statement(&mut self, location: SourceLocation) -> Result<Stmt> {
        self.expect(&TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RightParen)?;
        self.skip_newlines();

        let then_branch = Box::new(self.parse_statement()?);

        // Skip terminators (semicolons and newlines) before checking for else
        self.skip_terminators();
        let else_branch = if self.match_token(&TokenKind::Else) {
            self.skip_newlines();
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
            location,
        })
    }

    fn parse_while_statement(&mut self, location: SourceLocation) -> Result<Stmt> {
        self.expect(&TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RightParen)?;
        self.skip_newlines();

        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::While {
            condition,
            body,
            location,
        })
    }

    fn parse_for_statement(&mut self, location: SourceLocation) -> Result<Stmt> {
        self.expect(&TokenKind::LeftParen)?;

        // Check for for-in loop: for (var in array)
        if let Some(TokenKind::Identifier(name)) = self.peek_kind() {
            let name = name.clone();
            let saved_pos = self.current;
            self.advance();

            if self.match_token(&TokenKind::In) {
                let array = self.expect_identifier()?;
                self.expect(&TokenKind::RightParen)?;
                self.skip_newlines();
                let body = Box::new(self.parse_statement()?);

                return Ok(Stmt::ForIn {
                    var: name,
                    array,
                    body,
                    location,
                });
            }

            // Not a for-in, backtrack
            self.current = saved_pos;
        }

        // C-style for loop
        let init = if !self.check(&TokenKind::Semicolon) {
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };
        self.expect(&TokenKind::Semicolon)?;

        let condition = if !self.check(&TokenKind::Semicolon) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(&TokenKind::Semicolon)?;

        let update = if !self.check(&TokenKind::RightParen) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(&TokenKind::RightParen)?;
        self.skip_newlines();

        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::For {
            init,
            condition,
            update,
            body,
            location,
        })
    }

    fn parse_do_while_statement(&mut self, location: SourceLocation) -> Result<Stmt> {
        self.skip_newlines();
        let body = Box::new(self.parse_statement()?);
        self.skip_newlines();
        self.expect(&TokenKind::While)?;
        self.expect(&TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RightParen)?;

        Ok(Stmt::DoWhile {
            body,
            condition,
            location,
        })
    }

    fn parse_print_statement(&mut self, location: SourceLocation) -> Result<Stmt> {
        let mut args = Vec::new();

        // Check for arguments (print without args prints $0)
        if self.can_start_expression() && !self.check(&TokenKind::Greater) && !self.check(&TokenKind::Append) && !self.check(&TokenKind::Pipe) {
            args.push(self.parse_print_arg()?);
            while self.match_token(&TokenKind::Comma) {
                args.push(self.parse_print_arg()?);
            }
        }

        // Check for output redirection
        let output = self.parse_output_redirect()?;

        Ok(Stmt::Print {
            args,
            output,
            location,
        })
    }

    fn parse_printf_statement(&mut self, location: SourceLocation) -> Result<Stmt> {
        let format = self.parse_print_arg()?;
        let mut args = Vec::new();

        while self.match_token(&TokenKind::Comma) {
            args.push(self.parse_print_arg()?);
        }

        let output = self.parse_output_redirect()?;

        Ok(Stmt::Printf {
            format,
            args,
            output,
            location,
        })
    }

    fn parse_print_arg(&mut self) -> Result<Expr> {
        // Print arguments don't include comparison operators with > at the top level
        // because > is used for output redirection. We parse up to concatenation level.
        self.parse_print_ternary()
    }

    fn parse_print_ternary(&mut self) -> Result<Expr> {
        let expr = self.parse_print_or()?;

        if self.match_token(&TokenKind::Question) {
            let location = self.current_location();
            let then_expr = self.parse_print_ternary()?;
            self.expect(&TokenKind::Colon)?;
            let else_expr = self.parse_print_ternary()?;
            return Ok(Expr::Ternary {
                condition: Box::new(expr),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
                location,
            });
        }

        Ok(expr)
    }

    fn parse_print_or(&mut self) -> Result<Expr> {
        let mut expr = self.parse_print_and()?;

        while self.match_token(&TokenKind::Or) {
            let location = self.current_location();
            let right = self.parse_print_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Or,
                right: Box::new(right),
                location,
            };
        }

        Ok(expr)
    }

    fn parse_print_and(&mut self) -> Result<Expr> {
        let mut expr = self.parse_print_in()?;

        while self.match_token(&TokenKind::And) {
            let location = self.current_location();
            let right = self.parse_print_in()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::And,
                right: Box::new(right),
                location,
            };
        }

        Ok(expr)
    }

    fn parse_print_in(&mut self) -> Result<Expr> {
        let expr = self.parse_print_match()?;

        if self.match_token(&TokenKind::In) {
            let location = self.current_location();
            let array = self.expect_identifier()?;
            return Ok(Expr::InArray {
                key: vec![expr],
                array,
                location,
            });
        }

        Ok(expr)
    }

    fn parse_print_match(&mut self) -> Result<Expr> {
        let expr = self.parse_print_comparison()?;

        let location = self.current_location();
        if self.match_token(&TokenKind::Match) {
            let pattern = self.parse_print_comparison()?;
            return Ok(Expr::Match {
                expr: Box::new(expr),
                pattern: Box::new(pattern),
                negated: false,
                location,
            });
        }

        if self.match_token(&TokenKind::NotMatch) {
            let pattern = self.parse_print_comparison()?;
            return Ok(Expr::Match {
                expr: Box::new(expr),
                pattern: Box::new(pattern),
                negated: true,
                location,
            });
        }

        Ok(expr)
    }

    fn parse_print_comparison(&mut self) -> Result<Expr> {
        let mut expr = self.parse_concat()?;

        // For print args, we DON'T consume > or >> because they're used for output redirection
        // But we DO handle >=, <, <=, ==, !=
        loop {
            let location = self.current_location();
            let op = if self.match_token(&TokenKind::Less) {
                BinaryOp::Lt
            } else if self.match_token(&TokenKind::LessEqual) {
                BinaryOp::Le
            } else if self.match_token(&TokenKind::GreaterEqual) {
                BinaryOp::Ge
            } else if self.match_token(&TokenKind::Equal) {
                BinaryOp::Eq
            } else if self.match_token(&TokenKind::NotEqual) {
                BinaryOp::Ne
            } else {
                // Don't consume > or >> for print arguments (used for output redirection)
                break;
            };

            let right = self.parse_concat()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                location,
            };
        }

        Ok(expr)
    }

    fn parse_output_redirect(&mut self) -> Result<Option<OutputRedirect>> {
        if self.match_token(&TokenKind::Greater) {
            let target = self.parse_print_arg()?;
            Ok(Some(OutputRedirect::Truncate(target)))
        } else if self.match_token(&TokenKind::Append) {
            let target = self.parse_print_arg()?;
            Ok(Some(OutputRedirect::Append(target)))
        } else if self.match_token(&TokenKind::Pipe) {
            let target = self.parse_print_arg()?;
            Ok(Some(OutputRedirect::Pipe(target)))
        } else {
            Ok(None)
        }
    }

    /// Parse an expression
    fn parse_expression(&mut self) -> Result<Expr> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr> {
        let expr = self.parse_ternary()?;

        // Check for assignment operators
        let location = self.current_location();
        let op = if self.match_token(&TokenKind::Assign) {
            Some(AssignOp::Assign)
        } else if self.match_token(&TokenKind::PlusAssign) {
            Some(AssignOp::AddAssign)
        } else if self.match_token(&TokenKind::MinusAssign) {
            Some(AssignOp::SubAssign)
        } else if self.match_token(&TokenKind::StarAssign) {
            Some(AssignOp::MulAssign)
        } else if self.match_token(&TokenKind::SlashAssign) {
            Some(AssignOp::DivAssign)
        } else if self.match_token(&TokenKind::PercentAssign) {
            Some(AssignOp::ModAssign)
        } else if self.match_token(&TokenKind::CaretAssign) {
            Some(AssignOp::PowAssign)
        } else {
            None
        };

        if let Some(op) = op {
            let value = self.parse_assignment()?;
            return Ok(Expr::Assign {
                target: Box::new(expr),
                op,
                value: Box::new(value),
                location,
            });
        }

        Ok(expr)
    }

    fn parse_ternary(&mut self) -> Result<Expr> {
        let expr = self.parse_or()?;

        if self.match_token(&TokenKind::Question) {
            let location = self.current_location();
            let then_expr = self.parse_expression()?;
            self.expect(&TokenKind::Colon)?;
            let else_expr = self.parse_ternary()?;
            return Ok(Expr::Ternary {
                condition: Box::new(expr),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
                location,
            });
        }

        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut expr = self.parse_and()?;

        while self.match_token(&TokenKind::Or) {
            let location = self.current_location();
            let right = self.parse_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Or,
                right: Box::new(right),
                location,
            };
        }

        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut expr = self.parse_in()?;

        while self.match_token(&TokenKind::And) {
            let location = self.current_location();
            let right = self.parse_in()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::And,
                right: Box::new(right),
                location,
            };
        }

        Ok(expr)
    }

    fn parse_in(&mut self) -> Result<Expr> {
        let expr = self.parse_match()?;

        // Check for "in" (array membership)
        // Format: (expr) in array or expr in array
        if self.match_token(&TokenKind::In) {
            let location = self.current_location();
            let array = self.expect_identifier()?;
            return Ok(Expr::InArray {
                key: vec![expr],
                array,
                location,
            });
        }

        Ok(expr)
    }

    fn parse_match(&mut self) -> Result<Expr> {
        let expr = self.parse_comparison()?;

        let location = self.current_location();
        if self.match_token(&TokenKind::Match) {
            let pattern = self.parse_comparison()?;
            return Ok(Expr::Match {
                expr: Box::new(expr),
                pattern: Box::new(pattern),
                negated: false,
                location,
            });
        }

        if self.match_token(&TokenKind::NotMatch) {
            let pattern = self.parse_comparison()?;
            return Ok(Expr::Match {
                expr: Box::new(expr),
                pattern: Box::new(pattern),
                negated: true,
                location,
            });
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr> {
        let mut expr = self.parse_concat()?;

        loop {
            let location = self.current_location();
            let op = if self.match_token(&TokenKind::Less) {
                BinaryOp::Lt
            } else if self.match_token(&TokenKind::LessEqual) {
                BinaryOp::Le
            } else if self.match_token(&TokenKind::Greater) {
                BinaryOp::Gt
            } else if self.match_token(&TokenKind::GreaterEqual) {
                BinaryOp::Ge
            } else if self.match_token(&TokenKind::Equal) {
                BinaryOp::Eq
            } else if self.match_token(&TokenKind::NotEqual) {
                BinaryOp::Ne
            } else {
                break;
            };

            let right = self.parse_concat()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                location,
            };
        }

        Ok(expr)
    }

    fn parse_concat(&mut self) -> Result<Expr> {
        let mut expr = self.parse_additive()?;

        // Concatenation is implicit between adjacent expressions
        // But we need to be careful about operators that could follow
        while self.can_start_concat_operand() {
            let right = self.parse_additive()?;
            let location = expr.location();
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Concat,
                right: Box::new(right),
                location,
            };
        }

        Ok(expr)
    }

    fn can_start_concat_operand(&mut self) -> bool {
        if let Some(kind) = self.peek_kind() {
            matches!(
                kind,
                TokenKind::Number(_)
                    | TokenKind::String(_)
                    | TokenKind::Identifier(_)
                    | TokenKind::Dollar
                    | TokenKind::LeftParen
                    | TokenKind::Not
                    | TokenKind::Increment
                    | TokenKind::Decrement
            )
        } else {
            false
        }
    }

    fn parse_additive(&mut self) -> Result<Expr> {
        let mut expr = self.parse_multiplicative()?;

        loop {
            let location = self.current_location();
            let op = if self.match_token(&TokenKind::Plus) {
                BinaryOp::Add
            } else if self.match_token(&TokenKind::Minus) {
                BinaryOp::Sub
            } else {
                break;
            };

            let right = self.parse_multiplicative()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                location,
            };
        }

        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr> {
        let mut expr = self.parse_power()?;

        loop {
            let location = self.current_location();
            let op = if self.match_token(&TokenKind::Star) {
                BinaryOp::Mul
            } else if self.match_token(&TokenKind::Slash) {
                BinaryOp::Div
            } else if self.match_token(&TokenKind::Percent) {
                BinaryOp::Mod
            } else {
                break;
            };

            let right = self.parse_power()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                location,
            };
        }

        Ok(expr)
    }

    fn parse_power(&mut self) -> Result<Expr> {
        let expr = self.parse_unary()?;

        // Exponentiation is right-associative
        if self.match_token(&TokenKind::Caret) {
            let location = self.current_location();
            let right = self.parse_power()?;
            return Ok(Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Pow,
                right: Box::new(right),
                location,
            });
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        let location = self.current_location();

        if self.match_token(&TokenKind::Not) {
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
                location,
            });
        }

        if self.match_token(&TokenKind::Minus) {
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
                location,
            });
        }

        if self.match_token(&TokenKind::Plus) {
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Pos,
                operand: Box::new(operand),
                location,
            });
        }

        if self.match_token(&TokenKind::Increment) {
            let operand = self.parse_unary()?;
            return Ok(Expr::PreIncrement(Box::new(operand), location));
        }

        if self.match_token(&TokenKind::Decrement) {
            let operand = self.parse_unary()?;
            return Ok(Expr::PreDecrement(Box::new(operand), location));
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_field()?;

        loop {
            let location = self.current_location();

            if self.match_token(&TokenKind::Increment) {
                expr = Expr::PostIncrement(Box::new(expr), location);
            } else if self.match_token(&TokenKind::Decrement) {
                expr = Expr::PostDecrement(Box::new(expr), location);
            } else if self.match_token(&TokenKind::LeftBracket) {
                // Array access
                if let Expr::Var(name, _) = expr {
                    let mut indices = vec![self.parse_expression()?];
                    while self.match_token(&TokenKind::Comma) {
                        indices.push(self.parse_expression()?);
                    }
                    self.expect(&TokenKind::RightBracket)?;
                    expr = Expr::ArrayAccess {
                        array: name,
                        indices,
                        location,
                    };
                } else {
                    return Err(Error::parser("array access requires variable name", location.line, location.column));
                }
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_field(&mut self) -> Result<Expr> {
        if self.match_token(&TokenKind::Dollar) {
            let location = self.current_location();
            let expr = self.parse_field()?;
            return Ok(Expr::Field(Box::new(expr), location));
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        let location = self.current_location();

        // Number literal
        if let Some(TokenKind::Number(n)) = self.peek_kind() {
            let n = *n;
            self.advance();
            return Ok(Expr::Number(n, location));
        }

        // String literal
        if let Some(TokenKind::String(s)) = self.peek_kind() {
            let s = s.clone();
            self.advance();
            return Ok(Expr::String(s, location));
        }

        // Regex literal
        if let Some(TokenKind::Regex(r)) = self.peek_kind() {
            let r = r.clone();
            self.advance();
            return Ok(Expr::Regex(r, location));
        }

        // Identifier (variable or function call)
        if let Some(TokenKind::Identifier(name)) = self.peek_kind() {
            let name = name.clone();
            self.advance();

            // Check for function call
            if self.match_token(&TokenKind::LeftParen) {
                let mut args = Vec::new();
                if !self.check(&TokenKind::RightParen) {
                    args.push(self.parse_expression()?);
                    while self.match_token(&TokenKind::Comma) {
                        args.push(self.parse_expression()?);
                    }
                }
                self.expect(&TokenKind::RightParen)?;
                return Ok(Expr::Call {
                    name,
                    args,
                    location,
                });
            }

            return Ok(Expr::Var(name, location));
        }

        // Getline
        if self.match_token(&TokenKind::Getline) {
            let var = if let Some(TokenKind::Identifier(name)) = self.peek_kind() {
                let name = name.clone();
                self.advance();
                Some(name)
            } else {
                None
            };

            let input = if self.match_token(&TokenKind::Less) {
                Some(GetlineInput::File(Box::new(self.parse_primary()?)))
            } else {
                None
            };

            return Ok(Expr::Getline {
                var,
                input,
                location,
            });
        }

        // Parenthesized expression
        if self.match_token(&TokenKind::LeftParen) {
            let expr = self.parse_expression()?;
            self.expect(&TokenKind::RightParen)?;
            return Ok(Expr::Group(Box::new(expr), location));
        }

        Err(Error::parser(
            format!("unexpected token {:?}", self.peek_kind()),
            location.line,
            location.column,
        ))
    }

    // ===== Helper methods =====

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.current).map(|t| &t.kind)
    }

    fn current_location(&self) -> SourceLocation {
        self.tokens
            .get(self.current)
            .map(|t| t.location)
            .unwrap_or(SourceLocation::new(0, 0))
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek_kind(), None | Some(TokenKind::Eof))
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek_kind().map(|k| std::mem::discriminant(k) == std::mem::discriminant(kind)).unwrap_or(false)
    }

    fn advance(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.tokens.get(self.current - 1)
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<&Token> {
        if self.check(kind) {
            Ok(self.advance().unwrap())
        } else {
            let loc = self.current_location();
            Err(Error::parser(
                format!("expected {:?}, found {:?}", kind, self.peek_kind()),
                loc.line,
                loc.column,
            ))
        }
    }

    fn expect_identifier(&mut self) -> Result<String> {
        if let Some(TokenKind::Identifier(name)) = self.peek_kind() {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            let loc = self.current_location();
            Err(Error::parser(
                format!("expected identifier, found {:?}", self.peek_kind()),
                loc.line,
                loc.column,
            ))
        }
    }

    fn skip_newlines(&mut self) {
        while self.match_token(&TokenKind::Newline) {}
    }

    fn skip_terminators(&mut self) {
        while self.match_token(&TokenKind::Newline) || self.match_token(&TokenKind::Semicolon) {}
    }

    fn can_start_expression(&self) -> bool {
        self.peek_kind().map(|k| k.can_start_expression()).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(source: &str) -> Result<Program> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_simple_print() {
        let program = parse(r#"{ print "hello" }"#).unwrap();
        assert_eq!(program.rules.len(), 1);
    }

    #[test]
    fn test_begin_end() {
        let program = parse(r#"BEGIN { x = 1 } END { print x }"#).unwrap();
        assert_eq!(program.rules.len(), 2);
        assert!(matches!(program.rules[0].pattern, Some(Pattern::Begin)));
        assert!(matches!(program.rules[1].pattern, Some(Pattern::End)));
    }

    #[test]
    fn test_regex_pattern() {
        let program = parse(r#"/foo/ { print }"#).unwrap();
        assert_eq!(program.rules.len(), 1);
        assert!(matches!(
            &program.rules[0].pattern,
            Some(Pattern::Regex(r)) if r == "foo"
        ));
    }

    #[test]
    fn test_arithmetic() {
        let program = parse(r#"{ x = 1 + 2 * 3 }"#).unwrap();
        assert_eq!(program.rules.len(), 1);
    }

    #[test]
    fn test_function_def() {
        let program = parse(r#"function add(a, b) { return a + b }"#).unwrap();
        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "add");
        assert_eq!(program.functions[0].params, vec!["a", "b"]);
    }
}
