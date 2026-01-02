use crate::error::SourceLocation;

/// All token types in AWK
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(f64),
    String(String),
    Regex(String),

    // Identifiers and keywords
    Identifier(String),
    Begin,
    End,
    BeginFile, // gawk extension
    EndFile,   // gawk extension
    If,
    Else,
    While,
    For,
    Do,
    Break,
    Continue,
    Function,
    Return,
    Delete,
    Exit,
    Next,
    Nextfile,
    Getline,
    Print,
    Printf,
    In,

    // Operators - Arithmetic
    Plus,    // +
    Minus,   // -
    Star,    // *
    Slash,   // /
    Percent, // %
    Caret,   // ^

    // Operators - Comparison
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=
    Equal,        // ==
    NotEqual,     // !=

    // Operators - Logical
    And, // &&
    Or,  // ||
    Not, // !

    // Operators - Regex
    Match,    // ~
    NotMatch, // !~

    // Operators - Assignment
    Assign,        // =
    PlusAssign,    // +=
    MinusAssign,   // -=
    StarAssign,    // *=
    SlashAssign,   // /=
    PercentAssign, // %=
    CaretAssign,   // ^=

    // Operators - Increment/Decrement
    Increment, // ++
    Decrement, // --

    // Special operators
    Dollar,   // $ (field access)
    Question, // ?
    Colon,    // :
    Pipe,     // |
    Append,   // >>

    // Delimiters
    LeftParen,    // (
    RightParen,   // )
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]
    Semicolon,    // ;
    Comma,        // ,
    Newline,      // \n (significant in AWK)

    // End of file
    Eof,
}

impl TokenKind {
    /// Check if this token is a keyword
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::Begin
                | TokenKind::End
                | TokenKind::If
                | TokenKind::Else
                | TokenKind::While
                | TokenKind::For
                | TokenKind::Do
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Function
                | TokenKind::Return
                | TokenKind::Delete
                | TokenKind::Exit
                | TokenKind::Next
                | TokenKind::Nextfile
                | TokenKind::Getline
                | TokenKind::Print
                | TokenKind::Printf
                | TokenKind::In
        )
    }

    /// Check if this token can start an expression
    pub fn can_start_expression(&self) -> bool {
        matches!(
            self,
            TokenKind::Number(_)
                | TokenKind::String(_)
                | TokenKind::Regex(_)
                | TokenKind::Identifier(_)
                | TokenKind::LeftParen
                | TokenKind::Dollar
                | TokenKind::Not
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Increment
                | TokenKind::Decrement
                | TokenKind::Getline
        )
    }

    /// Check if this token produces a value (for regex vs division disambiguation)
    pub fn produces_value(&self) -> bool {
        matches!(
            self,
            TokenKind::Number(_)
                | TokenKind::String(_)
                | TokenKind::Identifier(_)
                | TokenKind::RightParen
                | TokenKind::RightBracket
                | TokenKind::Increment
                | TokenKind::Decrement
        )
    }
}

/// A token with its location in the source
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub location: SourceLocation,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Self {
            kind,
            location: SourceLocation::new(line, column),
        }
    }
}

/// Map keyword strings to token kinds
pub fn keyword_to_token(s: &str) -> Option<TokenKind> {
    match s {
        "BEGIN" => Some(TokenKind::Begin),
        "END" => Some(TokenKind::End),
        "BEGINFILE" => Some(TokenKind::BeginFile),
        "ENDFILE" => Some(TokenKind::EndFile),
        "if" => Some(TokenKind::If),
        "else" => Some(TokenKind::Else),
        "while" => Some(TokenKind::While),
        "for" => Some(TokenKind::For),
        "do" => Some(TokenKind::Do),
        "break" => Some(TokenKind::Break),
        "continue" => Some(TokenKind::Continue),
        "function" => Some(TokenKind::Function),
        "return" => Some(TokenKind::Return),
        "delete" => Some(TokenKind::Delete),
        "exit" => Some(TokenKind::Exit),
        "next" => Some(TokenKind::Next),
        "nextfile" => Some(TokenKind::Nextfile),
        "getline" => Some(TokenKind::Getline),
        "print" => Some(TokenKind::Print),
        "printf" => Some(TokenKind::Printf),
        "in" => Some(TokenKind::In),
        _ => None,
    }
}
