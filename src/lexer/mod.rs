mod tokens;

pub use tokens::{keyword_to_token, Token, TokenKind};

use crate::error::{Error, Result};

/// AWK lexer that tokenizes source code
pub struct Lexer<'a> {
    source: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    line: usize,
    column: usize,
    last_token_produces_value: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            line: 1,
            column: 1,
            last_token_produces_value: false,
        }
    }

    /// Tokenize the entire source, returning all tokens
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        // Pre-allocate based on source length (rough estimate: 1 token per 4 chars)
        let estimated_tokens = self.source.len() / 4 + 1;
        let mut tokens = Vec::with_capacity(estimated_tokens.min(1024));

        loop {
            let token = self.next_token()?;
            let is_eof = matches!(token.kind, TokenKind::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    /// Get the next token from the source
    pub fn next_token(&mut self) -> Result<Token> {
        self.skip_whitespace_and_comments();

        let (line, col) = (self.line, self.column);

        let Some((_pos, ch)) = self.peek_char() else {
            return Ok(Token::new(TokenKind::Eof, line, col));
        };

        let token = match ch {
            // Newlines are significant in AWK
            '\n' => {
                self.advance();
                Token::new(TokenKind::Newline, line, col)
            }

            // String literals
            '"' => self.scan_string()?,

            // Regex or division - depends on context
            '/' => {
                if self.last_token_produces_value {
                    self.advance();
                    if self.peek_char_is('=') {
                        self.advance();
                        Token::new(TokenKind::SlashAssign, line, col)
                    } else {
                        Token::new(TokenKind::Slash, line, col)
                    }
                } else {
                    self.scan_regex()?
                }
            }

            // Numbers
            '0'..='9' | '.' if ch == '.' && self.peek_next_is_digit() => self.scan_number()?,
            '0'..='9' => self.scan_number()?,

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => self.scan_identifier()?,

            // Operators and delimiters
            '+' => {
                self.advance();
                if self.peek_char_is('+') {
                    self.advance();
                    Token::new(TokenKind::Increment, line, col)
                } else if self.peek_char_is('=') {
                    self.advance();
                    Token::new(TokenKind::PlusAssign, line, col)
                } else {
                    Token::new(TokenKind::Plus, line, col)
                }
            }
            '-' => {
                self.advance();
                if self.peek_char_is('-') {
                    self.advance();
                    Token::new(TokenKind::Decrement, line, col)
                } else if self.peek_char_is('=') {
                    self.advance();
                    Token::new(TokenKind::MinusAssign, line, col)
                } else {
                    Token::new(TokenKind::Minus, line, col)
                }
            }
            '*' => {
                self.advance();
                if self.peek_char_is('=') {
                    self.advance();
                    Token::new(TokenKind::StarAssign, line, col)
                } else {
                    Token::new(TokenKind::Star, line, col)
                }
            }
            '%' => {
                self.advance();
                if self.peek_char_is('=') {
                    self.advance();
                    Token::new(TokenKind::PercentAssign, line, col)
                } else {
                    Token::new(TokenKind::Percent, line, col)
                }
            }
            '^' => {
                self.advance();
                if self.peek_char_is('=') {
                    self.advance();
                    Token::new(TokenKind::CaretAssign, line, col)
                } else {
                    Token::new(TokenKind::Caret, line, col)
                }
            }
            '<' => {
                self.advance();
                if self.peek_char_is('=') {
                    self.advance();
                    Token::new(TokenKind::LessEqual, line, col)
                } else {
                    Token::new(TokenKind::Less, line, col)
                }
            }
            '>' => {
                self.advance();
                if self.peek_char_is('=') {
                    self.advance();
                    Token::new(TokenKind::GreaterEqual, line, col)
                } else if self.peek_char_is('>') {
                    self.advance();
                    Token::new(TokenKind::Append, line, col)
                } else {
                    Token::new(TokenKind::Greater, line, col)
                }
            }
            '=' => {
                self.advance();
                if self.peek_char_is('=') {
                    self.advance();
                    Token::new(TokenKind::Equal, line, col)
                } else {
                    Token::new(TokenKind::Assign, line, col)
                }
            }
            '!' => {
                self.advance();
                if self.peek_char_is('=') {
                    self.advance();
                    Token::new(TokenKind::NotEqual, line, col)
                } else if self.peek_char_is('~') {
                    self.advance();
                    Token::new(TokenKind::NotMatch, line, col)
                } else {
                    Token::new(TokenKind::Not, line, col)
                }
            }
            '~' => {
                self.advance();
                Token::new(TokenKind::Match, line, col)
            }
            '&' => {
                self.advance();
                if self.peek_char_is('&') {
                    self.advance();
                    Token::new(TokenKind::And, line, col)
                } else {
                    return Err(Error::lexer("unexpected '&', did you mean '&&'?", line, col));
                }
            }
            '|' => {
                self.advance();
                if self.peek_char_is('|') {
                    self.advance();
                    Token::new(TokenKind::Or, line, col)
                } else {
                    Token::new(TokenKind::Pipe, line, col)
                }
            }
            '$' => {
                self.advance();
                Token::new(TokenKind::Dollar, line, col)
            }
            '?' => {
                self.advance();
                Token::new(TokenKind::Question, line, col)
            }
            ':' => {
                self.advance();
                Token::new(TokenKind::Colon, line, col)
            }
            '(' => {
                self.advance();
                Token::new(TokenKind::LeftParen, line, col)
            }
            ')' => {
                self.advance();
                Token::new(TokenKind::RightParen, line, col)
            }
            '{' => {
                self.advance();
                Token::new(TokenKind::LeftBrace, line, col)
            }
            '}' => {
                self.advance();
                Token::new(TokenKind::RightBrace, line, col)
            }
            '[' => {
                self.advance();
                Token::new(TokenKind::LeftBracket, line, col)
            }
            ']' => {
                self.advance();
                Token::new(TokenKind::RightBracket, line, col)
            }
            ';' => {
                self.advance();
                Token::new(TokenKind::Semicolon, line, col)
            }
            ',' => {
                self.advance();
                Token::new(TokenKind::Comma, line, col)
            }

            _ => {
                return Err(Error::lexer(
                    format!("unexpected character '{}'", ch),
                    line,
                    col,
                ));
            }
        };

        self.last_token_produces_value = token.kind.produces_value();
        Ok(token)
    }

    fn peek_char(&mut self) -> Option<(usize, char)> {
        self.chars.peek().copied()
    }

    fn peek_char_is(&mut self, expected: char) -> bool {
        self.chars.peek().map(|(_, c)| *c == expected).unwrap_or(false)
    }

    fn peek_next_is_digit(&self) -> bool {
        let mut chars = self.chars.clone();
        chars.next(); // skip current
        chars.next().map(|(_, c)| c.is_ascii_digit()).unwrap_or(false)
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        let result = self.chars.next();
        if let Some((_, ch)) = result {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        result
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek_char() {
                Some((_, ' ' | '\t' | '\r')) => {
                    self.advance();
                }
                Some((_, '\\')) => {
                    // Line continuation
                    let mut chars = self.chars.clone();
                    chars.next();
                    if chars.peek().map(|(_, c)| *c == '\n').unwrap_or(false) {
                        self.advance(); // consume backslash
                        self.advance(); // consume newline
                    } else {
                        break;
                    }
                }
                Some((_, '#')) => {
                    // Comment - skip to end of line
                    while let Some((_, ch)) = self.peek_char() {
                        if ch == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn scan_string(&mut self) -> Result<Token> {
        let (line, col) = (self.line, self.column);
        self.advance(); // consume opening quote

        let mut value = String::new();

        loop {
            match self.advance() {
                Some((_, '"')) => break,
                Some((_, '\\')) => {
                    // Escape sequence
                    match self.advance() {
                        Some((_, 'n')) => value.push('\n'),
                        Some((_, 't')) => value.push('\t'),
                        Some((_, 'r')) => value.push('\r'),
                        Some((_, 'b')) => value.push('\x08'),
                        Some((_, 'f')) => value.push('\x0C'),
                        Some((_, 'a')) => value.push('\x07'),
                        Some((_, 'v')) => value.push('\x0B'),
                        Some((_, '\\')) => value.push('\\'),
                        Some((_, '"')) => value.push('"'),
                        Some((_, '/')) => value.push('/'),
                        Some((_, c)) => {
                            // Unknown escape, just use the character
                            value.push(c);
                        }
                        None => {
                            return Err(Error::lexer("unterminated string", line, col));
                        }
                    }
                }
                Some((_, '\n')) => {
                    return Err(Error::lexer("unterminated string (newline in string)", line, col));
                }
                Some((_, ch)) => value.push(ch),
                None => {
                    return Err(Error::lexer("unterminated string", line, col));
                }
            }
        }

        Ok(Token::new(TokenKind::String(value), line, col))
    }

    fn scan_regex(&mut self) -> Result<Token> {
        let (line, col) = (self.line, self.column);
        self.advance(); // consume opening slash

        let mut pattern = String::new();

        loop {
            match self.advance() {
                Some((_, '/')) => break,
                Some((_, '\\')) => {
                    // Escape next character in regex
                    pattern.push('\\');
                    if let Some((_, ch)) = self.advance() {
                        pattern.push(ch);
                    } else {
                        return Err(Error::lexer("unterminated regex", line, col));
                    }
                }
                Some((_, '\n')) => {
                    return Err(Error::lexer("unterminated regex (newline in regex)", line, col));
                }
                Some((_, ch)) => pattern.push(ch),
                None => {
                    return Err(Error::lexer("unterminated regex", line, col));
                }
            }
        }

        Ok(Token::new(TokenKind::Regex(pattern), line, col))
    }

    fn scan_number(&mut self) -> Result<Token> {
        let (line, col) = (self.line, self.column);
        let start_pos = self.chars.peek().map(|(pos, _)| *pos).unwrap_or(0);
        let mut end_pos = start_pos;

        // Integer part
        while let Some((pos, ch)) = self.peek_char() {
            if ch.is_ascii_digit() {
                end_pos = pos + 1;
                self.advance();
            } else {
                break;
            }
        }

        // Decimal part
        if self.peek_char_is('.') {
            self.advance();
            end_pos += 1;

            while let Some((pos, ch)) = self.peek_char() {
                if ch.is_ascii_digit() {
                    end_pos = pos + 1;
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Exponent part
        if let Some((_, 'e' | 'E')) = self.peek_char() {
            self.advance();
            end_pos += 1;

            if let Some((_, '+' | '-')) = self.peek_char() {
                self.advance();
                end_pos += 1;
            }

            while let Some((pos, ch)) = self.peek_char() {
                if ch.is_ascii_digit() {
                    end_pos = pos + 1;
                    self.advance();
                } else {
                    break;
                }
            }
        }

        let number_str = &self.source[start_pos..end_pos];
        let value: f64 = number_str
            .parse()
            .map_err(|_| Error::lexer(format!("invalid number '{}'", number_str), line, col))?;

        Ok(Token::new(TokenKind::Number(value), line, col))
    }

    fn scan_identifier(&mut self) -> Result<Token> {
        let (line, col) = (self.line, self.column);
        let start_pos = self.chars.peek().map(|(pos, _)| *pos).unwrap_or(0);
        let mut end_pos = start_pos;

        while let Some((pos, ch)) = self.peek_char() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                end_pos = pos + 1;
                self.advance();
            } else {
                break;
            }
        }

        let ident = &self.source[start_pos..end_pos];

        // Check if it's a keyword
        let kind = keyword_to_token(ident).unwrap_or_else(|| TokenKind::Identifier(ident.to_string()));

        Ok(Token::new(kind, line, col))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokens() {
        // Test operators in context where / is division (after identifier)
        let mut lexer = Lexer::new("x + y - z * w / v % u");
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(tokens[1].kind, TokenKind::Plus));
        assert!(matches!(tokens[3].kind, TokenKind::Minus));
        assert!(matches!(tokens[5].kind, TokenKind::Star));
        assert!(matches!(tokens[7].kind, TokenKind::Slash));
        assert!(matches!(tokens[9].kind, TokenKind::Percent));
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("BEGIN END if else while for print");
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(tokens[0].kind, TokenKind::Begin));
        assert!(matches!(tokens[1].kind, TokenKind::End));
        assert!(matches!(tokens[2].kind, TokenKind::If));
        assert!(matches!(tokens[3].kind, TokenKind::Else));
        assert!(matches!(tokens[4].kind, TokenKind::While));
        assert!(matches!(tokens[5].kind, TokenKind::For));
        assert!(matches!(tokens[6].kind, TokenKind::Print));
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 3.14 1e10 2.5e-3");
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(tokens[0].kind, TokenKind::Number(n) if n == 42.0));
        assert!(matches!(tokens[1].kind, TokenKind::Number(n) if (n - 3.14).abs() < 0.001));
        assert!(matches!(tokens[2].kind, TokenKind::Number(n) if n == 1e10));
        assert!(matches!(tokens[3].kind, TokenKind::Number(n) if (n - 2.5e-3).abs() < 0.0001));
    }

    #[test]
    fn test_strings() {
        let mut lexer = Lexer::new(r#""hello" "world\n""#);
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(&tokens[0].kind, TokenKind::String(s) if s == "hello"));
        assert!(matches!(&tokens[1].kind, TokenKind::String(s) if s == "world\n"));
    }

    #[test]
    fn test_regex_vs_division() {
        // After identifier, / is division
        let mut lexer = Lexer::new("x / 2");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[1].kind, TokenKind::Slash));

        // At start, / begins a regex
        let mut lexer = Lexer::new("/pattern/");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Regex(s) if s == "pattern"));
    }

    #[test]
    fn test_line_tracking() {
        let mut lexer = Lexer::new("a\nb\nc");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].location.line, 1);
        assert_eq!(tokens[2].location.line, 2);
        assert_eq!(tokens[4].location.line, 3);
    }
}
