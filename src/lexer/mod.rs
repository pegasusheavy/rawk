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
                    match self.peek_char() {
                        Some((_, 'n')) => { self.advance(); value.push('\n'); }
                        Some((_, 't')) => { self.advance(); value.push('\t'); }
                        Some((_, 'r')) => { self.advance(); value.push('\r'); }
                        Some((_, 'b')) => { self.advance(); value.push('\x08'); }
                        Some((_, 'f')) => { self.advance(); value.push('\x0C'); }
                        Some((_, 'a')) => { self.advance(); value.push('\x07'); }
                        Some((_, 'v')) => { self.advance(); value.push('\x0B'); }
                        Some((_, '\\')) => { self.advance(); value.push('\\'); }
                        Some((_, '"')) => { self.advance(); value.push('"'); }
                        Some((_, '/')) => { self.advance(); value.push('/'); }
                        Some((_, 'x')) => {
                            // Hex escape: \xNN
                            self.advance(); // consume 'x'
                            let hex = self.read_hex_digits(2);
                            if let Some(ch) = u8::from_str_radix(&hex, 16).ok().map(|b| b as char) {
                                value.push(ch);
                            } else {
                                value.push_str("\\x");
                                value.push_str(&hex);
                            }
                        }
                        Some((_, c)) if c.is_ascii_digit() && c != '8' && c != '9' => {
                            // Octal escape: \NNN (1-3 octal digits)
                            let octal = self.read_octal_digits(3);
                            if let Some(ch) = u8::from_str_radix(&octal, 8).ok().map(|b| b as char) {
                                value.push(ch);
                            } else {
                                value.push('\\');
                                value.push_str(&octal);
                            }
                        }
                        Some((_, c)) => {
                            // Unknown escape, just use the character
                            self.advance();
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

    fn read_hex_digits(&mut self, max_count: usize) -> String {
        let mut result = String::new();
        for _ in 0..max_count {
            match self.peek_char() {
                Some((_, c)) if c.is_ascii_hexdigit() => {
                    self.advance();
                    result.push(c);
                }
                _ => break,
            }
        }
        result
    }

    fn read_octal_digits(&mut self, max_count: usize) -> String {
        let mut result = String::new();
        for _ in 0..max_count {
            match self.peek_char() {
                Some((_, c)) if c >= '0' && c <= '7' => {
                    self.advance();
                    result.push(c);
                }
                _ => break,
            }
        }
        result
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

    #[test]
    fn test_comparison_operators() {
        let mut lexer = Lexer::new("< <= > >= == !=");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Less));
        assert!(matches!(tokens[1].kind, TokenKind::LessEqual));
        assert!(matches!(tokens[2].kind, TokenKind::Greater));
        assert!(matches!(tokens[3].kind, TokenKind::GreaterEqual));
        assert!(matches!(tokens[4].kind, TokenKind::Equal));
        assert!(matches!(tokens[5].kind, TokenKind::NotEqual));
    }

    #[test]
    fn test_logical_operators() {
        let mut lexer = Lexer::new("&& || !");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::And));
        assert!(matches!(tokens[1].kind, TokenKind::Or));
        assert!(matches!(tokens[2].kind, TokenKind::Not));
    }

    #[test]
    fn test_regex_match_operators() {
        let mut lexer = Lexer::new("x ~ y x !~ y");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[1].kind, TokenKind::Match));
        assert!(matches!(tokens[4].kind, TokenKind::NotMatch));
    }

    #[test]
    fn test_assignment_operators() {
        // Put value-producing tokens before /= to avoid regex interpretation
        let mut lexer = Lexer::new("x = 1 x += 1 x -= 1 x *= 1 x /= 1 x %= 1 x ^= 1");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[1].kind, TokenKind::Assign));
        assert!(matches!(tokens[4].kind, TokenKind::PlusAssign));
        assert!(matches!(tokens[7].kind, TokenKind::MinusAssign));
        assert!(matches!(tokens[10].kind, TokenKind::StarAssign));
        assert!(matches!(tokens[13].kind, TokenKind::SlashAssign));
        assert!(matches!(tokens[16].kind, TokenKind::PercentAssign));
        assert!(matches!(tokens[19].kind, TokenKind::CaretAssign));
    }

    #[test]
    fn test_increment_decrement() {
        let mut lexer = Lexer::new("++ --");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Increment));
        assert!(matches!(tokens[1].kind, TokenKind::Decrement));
    }

    #[test]
    fn test_delimiters() {
        let mut lexer = Lexer::new("( ) { } [ ] ; ,");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::LeftParen));
        assert!(matches!(tokens[1].kind, TokenKind::RightParen));
        assert!(matches!(tokens[2].kind, TokenKind::LeftBrace));
        assert!(matches!(tokens[3].kind, TokenKind::RightBrace));
        assert!(matches!(tokens[4].kind, TokenKind::LeftBracket));
        assert!(matches!(tokens[5].kind, TokenKind::RightBracket));
        assert!(matches!(tokens[6].kind, TokenKind::Semicolon));
        assert!(matches!(tokens[7].kind, TokenKind::Comma));
    }

    #[test]
    fn test_special_operators() {
        let mut lexer = Lexer::new("$ ? : | >>");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Dollar));
        assert!(matches!(tokens[1].kind, TokenKind::Question));
        assert!(matches!(tokens[2].kind, TokenKind::Colon));
        assert!(matches!(tokens[3].kind, TokenKind::Pipe));
        assert!(matches!(tokens[4].kind, TokenKind::Append));
    }

    #[test]
    fn test_exponent() {
        let mut lexer = Lexer::new("x ^ 2");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[1].kind, TokenKind::Caret));
    }

    #[test]
    fn test_comments() {
        let mut lexer = Lexer::new("x # this is a comment\ny");
        let tokens = lexer.tokenize().unwrap();
        // Comment should be skipped
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(s) if s == "x"));
        assert!(matches!(tokens[1].kind, TokenKind::Newline));
        assert!(matches!(&tokens[2].kind, TokenKind::Identifier(s) if s == "y"));
    }

    #[test]
    fn test_line_continuation() {
        let mut lexer = Lexer::new("x \\\ny");
        let tokens = lexer.tokenize().unwrap();
        // Backslash-newline should be skipped
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(s) if s == "x"));
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "y"));
    }

    #[test]
    fn test_string_escapes() {
        let mut lexer = Lexer::new(r#""\t\r\n\b\f\a\v\\\"\/""#);
        let tokens = lexer.tokenize().unwrap();
        if let TokenKind::String(s) = &tokens[0].kind {
            assert!(s.contains('\t'));
            assert!(s.contains('\r'));
            assert!(s.contains('\n'));
            assert!(s.contains('\\'));
            assert!(s.contains('"'));
            assert!(s.contains('/'));
        } else {
            panic!("Expected string token");
        }
    }

    #[test]
    fn test_hex_escape() {
        let mut lexer = Lexer::new(r#""\x41\x42""#);
        let tokens = lexer.tokenize().unwrap();
        if let TokenKind::String(s) = &tokens[0].kind {
            assert_eq!(s, "AB");
        } else {
            panic!("Expected string token");
        }
    }

    #[test]
    fn test_octal_escape() {
        let mut lexer = Lexer::new(r#""\101\102""#);
        let tokens = lexer.tokenize().unwrap();
        if let TokenKind::String(s) = &tokens[0].kind {
            assert_eq!(s, "AB");
        } else {
            panic!("Expected string token");
        }
    }

    #[test]
    fn test_regex_with_escapes() {
        let mut lexer = Lexer::new(r#"/a\/b/"#);
        let tokens = lexer.tokenize().unwrap();
        if let TokenKind::Regex(s) = &tokens[0].kind {
            assert!(s.contains("\\/"));
        } else {
            panic!("Expected regex token");
        }
    }

    #[test]
    fn test_more_keywords() {
        let mut lexer = Lexer::new("do break continue function return delete exit next nextfile getline printf in BEGINFILE ENDFILE");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Do));
        assert!(matches!(tokens[1].kind, TokenKind::Break));
        assert!(matches!(tokens[2].kind, TokenKind::Continue));
        assert!(matches!(tokens[3].kind, TokenKind::Function));
        assert!(matches!(tokens[4].kind, TokenKind::Return));
        assert!(matches!(tokens[5].kind, TokenKind::Delete));
        assert!(matches!(tokens[6].kind, TokenKind::Exit));
        assert!(matches!(tokens[7].kind, TokenKind::Next));
        assert!(matches!(tokens[8].kind, TokenKind::Nextfile));
        assert!(matches!(tokens[9].kind, TokenKind::Getline));
        assert!(matches!(tokens[10].kind, TokenKind::Printf));
        assert!(matches!(tokens[11].kind, TokenKind::In));
        assert!(matches!(tokens[12].kind, TokenKind::BeginFile));
        assert!(matches!(tokens[13].kind, TokenKind::EndFile));
    }

    #[test]
    fn test_number_with_exponent() {
        let mut lexer = Lexer::new("1e+5 1E-5");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Number(n) if n == 1e5));
        assert!(matches!(tokens[1].kind, TokenKind::Number(n) if n == 1e-5));
    }

    #[test]
    fn test_decimal_starting_with_dot() {
        let mut lexer = Lexer::new(".5 .123");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Number(n) if (n - 0.5).abs() < 0.001));
        assert!(matches!(tokens[1].kind, TokenKind::Number(n) if (n - 0.123).abs() < 0.001));
    }

    #[test]
    fn test_unexpected_character_error() {
        let mut lexer = Lexer::new("@");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_unterminated_string_error() {
        let mut lexer = Lexer::new("\"unterminated");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_unterminated_regex_error() {
        let mut lexer = Lexer::new("/unterminated");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_string_with_newline_error() {
        let mut lexer = Lexer::new("\"hello\nworld\"");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_single_ampersand_error() {
        let mut lexer = Lexer::new("& ");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_eof_token() {
        let mut lexer = Lexer::new("");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_regex_with_escapes_complete() {
        let mut lexer = Lexer::new(r#"/\d+\.\d*/"#);
        let tokens = lexer.tokenize().unwrap();
        if let TokenKind::Regex(s) = &tokens[0].kind {
            assert!(s.contains(r"\d"));
        } else {
            panic!("Expected regex token");
        }
    }

    #[test]
    fn test_string_unknown_escape() {
        // Unknown escape sequences should just use the character
        let mut lexer = Lexer::new(r#""\q""#);
        let tokens = lexer.tokenize().unwrap();
        if let TokenKind::String(s) = &tokens[0].kind {
            assert_eq!(s, "q");
        } else {
            panic!("Expected string token");
        }
    }

    #[test]
    fn test_number_leading_dot() {
        let mut lexer = Lexer::new(".123 .5e2");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Number(n) if (n - 0.123).abs() < 0.001));
        assert!(matches!(tokens[1].kind, TokenKind::Number(n) if n == 50.0));
    }

    #[test]
    fn test_number_exponent_variations() {
        let mut lexer = Lexer::new("1e5 1E5 1e+5 1e-5 1.5e10");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Number(n) if n == 1e5));
        assert!(matches!(tokens[1].kind, TokenKind::Number(n) if n == 1e5));
        assert!(matches!(tokens[2].kind, TokenKind::Number(n) if n == 1e5));
        assert!(matches!(tokens[3].kind, TokenKind::Number(n) if n == 1e-5));
        assert!(matches!(tokens[4].kind, TokenKind::Number(n) if n == 1.5e10));
    }

    #[test]
    fn test_regex_after_comma() {
        // After comma, / should be regex
        let mut lexer = Lexer::new("gsub(/a/, /b/)");
        let tokens = lexer.tokenize().unwrap();
        // gsub ( /a/ , /b/ )
        assert!(matches!(&tokens[2].kind, TokenKind::Regex(s) if s == "a"));
        assert!(matches!(&tokens[4].kind, TokenKind::Regex(s) if s == "b"));
    }

    #[test]
    fn test_regex_after_operators() {
        // After various operators, / should be regex
        let mut lexer = Lexer::new("x ~ /a/ && /b/");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[2].kind, TokenKind::Regex(s) if s == "a"));
        assert!(matches!(&tokens[4].kind, TokenKind::Regex(s) if s == "b"));
    }

    #[test]
    fn test_multiple_newlines() {
        let mut lexer = Lexer::new("a\n\n\nb");
        let tokens = lexer.tokenize().unwrap();
        // Should have multiple newline tokens
        let newline_count = tokens.iter().filter(|t| matches!(t.kind, TokenKind::Newline)).count();
        assert!(newline_count >= 2);
    }

    #[test]
    fn test_comment_at_end() {
        let mut lexer = Lexer::new("x # comment at end");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(s) if s == "x"));
        assert!(matches!(tokens[1].kind, TokenKind::Eof));
    }

    #[test]
    fn test_identifier_with_underscore() {
        let mut lexer = Lexer::new("_var var_name my_func_2");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(s) if s == "_var"));
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "var_name"));
        assert!(matches!(&tokens[2].kind, TokenKind::Identifier(s) if s == "my_func_2"));
    }

    #[test]
    fn test_string_with_all_escapes() {
        let mut lexer = Lexer::new(r#""\b\f""#);
        let tokens = lexer.tokenize().unwrap();
        if let TokenKind::String(s) = &tokens[0].kind {
            assert!(s.contains('\x08'));  // backspace
            assert!(s.contains('\x0C'));  // form feed
        } else {
            panic!("Expected string token");
        }
    }

    #[test]
    fn test_invalid_hex_escape() {
        // Invalid hex should fall back gracefully
        let mut lexer = Lexer::new(r#""\xGG""#);
        let tokens = lexer.tokenize().unwrap();
        if let TokenKind::String(s) = &tokens[0].kind {
            // Should contain x since the hex parse failed
            assert!(s.contains("GG") || s.contains("x"));
        } else {
            panic!("Expected string token");
        }
    }

    #[test]
    fn test_single_pipe() {
        let mut lexer = Lexer::new("a | b");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[1].kind, TokenKind::Pipe));
    }

    #[test]
    fn test_double_pipe() {
        let mut lexer = Lexer::new("a || b");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[1].kind, TokenKind::Or));
    }

    #[test]
    fn test_colon() {
        let mut lexer = Lexer::new("a ? b : c");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[1].kind, TokenKind::Question));
        assert!(matches!(tokens[3].kind, TokenKind::Colon));
    }

    #[test]
    fn test_caret() {
        let mut lexer = Lexer::new("x ^ 2 x ^= 3");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[1].kind, TokenKind::Caret));
        assert!(matches!(tokens[4].kind, TokenKind::CaretAssign));
    }

    #[test]
    fn test_all_assignment_types() {
        let mut lexer = Lexer::new("a += 1 b -= 1 c *= 1 d %= 1");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[1].kind, TokenKind::PlusAssign));
        assert!(matches!(tokens[4].kind, TokenKind::MinusAssign));
        assert!(matches!(tokens[7].kind, TokenKind::StarAssign));
        assert!(matches!(tokens[10].kind, TokenKind::PercentAssign));
    }
}
