use std::collections::VecDeque;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Fn,
    Let,
    Const,
    If,
    Else,
    Elif,
    While,
    For,
    In,
    Class,
    PyImport,
    Return,
    True,
    False,
    Try,
    Catch,
    Finally,
    Raise,
    Trait,
    Super,
    Match,
    Case,
    Underscore,
    Async,
    Await,
    Not,
    Break,
    Continue,
    Null,
    Optional,
    And,
    Or,
    Import,
    From,


    // Primitive types
    IntType,
    FloatType,
    Float32Type,
    StringType,
    BoolType,
    VoidType,

    // Symbols & Operators
    Bang,
    Question,
    QuestionDot,
    FStringStart,
    FStringEnd,
    FStringText(String),
    FStringExprOpen,
    FStringExprClose,

    // Identifiers & Literals
    Identifier(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),

    // Symbols & Operators
    Plus,
    PlusEq,
    Minus,
    MinusEq,
    Star,
    StarEq,
    Pow,
    Slash,
    Percent,
    Assign,
    At,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Colon,
    OpenParen,
    CloseParen,
    Comma,
    Arrow,
    Dot,
    OpenBracket,
    CloseBracket,
    OpenBrace,
    CloseBrace,
    Pipe,

    // Layout
    Newline,
    Indent,
    Dedent,
    EOF,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Fn => write!(f, "fn"),
            Token::Let => write!(f, "let"),
            Token::Const => write!(f, "const"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::Elif => write!(f, "elif"),
            Token::While => write!(f, "while"),
            Token::For => write!(f, "for"),
            Token::In => write!(f, "in"),
            Token::Class => write!(f, "class"),
            Token::PyImport => write!(f, "pyimport"),
            Token::Return => write!(f, "return"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Try => write!(f, "try"),
            Token::Catch => write!(f, "catch"),
            Token::Finally => write!(f, "finally"),
            Token::Raise => write!(f, "raise"),
            Token::Trait => write!(f, "trait"),
            Token::Super => write!(f, "super"),
            Token::Match => write!(f, "match"),
            Token::Case => write!(f, "case"),
            Token::Underscore => write!(f, "_"),
                    Token::Async => write!(f, "async"),
            Token::Await => write!(f, "await"),
            Token::Not => write!(f, "not"),
            Token::Break => write!(f, "break"),
            Token::Continue => write!(f, "continue"),
            Token::Null => write!(f, "null"),
            Token::Optional => write!(f, "optional"),
            Token::And => write!(f, "and"),
            Token::Or => write!(f, "or"),
            Token::Import => write!(f, "import"),
            Token::From => write!(f, "from"),
            Token::IntType => write!(f, "int"),
            Token::FloatType => write!(f, "float"),
            Token::Float32Type => write!(f, "float32"),
            Token::StringType => write!(f, "string"),
            Token::BoolType => write!(f, "bool"),
            Token::VoidType => write!(f, "void"),
            Token::Identifier(s) => write!(f, "{}", s),
            Token::IntLiteral(n) => write!(f, "{}", n),
            Token::FloatLiteral(n) => write!(f, "{}", n),
            Token::StringLiteral(s) => write!(f, "\"{}\"", s),
            Token::Plus => write!(f, "+"),
            Token::PlusEq => write!(f, "+="),
            Token::Minus => write!(f, "-"),
            Token::MinusEq => write!(f, "-="),
            Token::Star => write!(f, "*"),
            Token::StarEq => write!(f, "*="),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Pow => write!(f, "**"),
            Token::Assign => write!(f, "="),
            Token::At => write!(f, "@"),
            Token::Eq => write!(f, "=="),
            Token::Ne => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Le => write!(f, "<="),
            Token::Gt => write!(f, ">"),
            Token::Ge => write!(f, ">="),
            Token::Colon => write!(f, ":"),
            Token::OpenParen => write!(f, "("),
            Token::CloseParen => write!(f, ")"),
            Token::Comma => write!(f, ","),
            Token::Arrow => write!(f, "->"),
            Token::Dot => write!(f, "."),
            Token::OpenBracket => write!(f, "["),
            Token::CloseBracket => write!(f, "]"),
            Token::OpenBrace => write!(f, "{{"),
            Token::CloseBrace => write!(f, "}}"),
            Token::Pipe => write!(f, "|>"),
            Token::Bang => write!(f, "!"),
            Token::Question => write!(f, "?"),
            Token::QuestionDot => write!(f, "?."),
            Token::FStringStart => write!(f, "f\""),
            Token::FStringEnd => write!(f, "\""),
            Token::FStringText(s) => write!(f, "{}", s),
            Token::FStringExprOpen => write!(f, "{{"),
            Token::FStringExprClose => write!(f, "}}"),
            Token::Newline => write!(f, "<NEWLINE>"),
            Token::Indent => write!(f, "<INDENT>"),
            Token::Dedent => write!(f, "<DEDENT>"),
            Token::EOF => write!(f, "<EOF>"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for SpannedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.token)
    }
}

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    indent_stack: Vec<usize>,
    pending_tokens: VecDeque<SpannedToken>,
    is_at_line_start: bool,
    paren_nesting: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            indent_stack: vec![0],
            pending_tokens: VecDeque::new(),
            is_at_line_start: true,
            paren_nesting: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        if self.pos < self.source.len() {
            Some(self.source[self.pos])
        } else {
            None
        }
    }

    fn peek_next(&self) -> Option<char> {
        if self.pos + 1 < self.source.len() {
            Some(self.source[self.pos + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<char> {
        if self.pos < self.source.len() {
            let ch = self.source[self.pos];
            self.pos += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            Some(ch)
        } else {
            None
        }
    }

    fn skip_whitespace_on_line(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn make_spanned(&self, token: Token) -> SpannedToken {
        SpannedToken {
            token,
            line: self.line,
            col: self.col,
        }
    }

    pub fn next_token(&mut self) -> Result<SpannedToken, String> {
        loop {
            // Drain pending queue
            if let Some(tok) = self.pending_tokens.pop_front() {
                return Ok(tok);
            }

            if self.pos >= self.source.len() {
                // EOF reached
                if self.indent_stack.len() > 1 {
                    while self.indent_stack.len() > 1 {
                        self.indent_stack.pop();
                        let d_tok = self.make_spanned(Token::Dedent);
                        self.pending_tokens.push_back(d_tok);
                    }
                    let eof_tok = self.make_spanned(Token::EOF);
                    self.pending_tokens.push_back(eof_tok);
                    continue;
                }
                return Ok(self.make_spanned(Token::EOF));
            }

            if self.is_at_line_start && self.paren_nesting == 0 {
                let mut indent_width = 0;
                let mut has_spaces = false;
                let mut has_tabs = false;

                // Count leading indentation whitespace
                while let Some(ch) = self.peek() {
                    if ch == ' ' {
                        indent_width += 1;
                        has_spaces = true;
                        self.advance();
                    } else if ch == '\t' {
                        indent_width += 4;
                        has_tabs = true;
                        self.advance();
                    } else {
                        break;
                    }
                }

                if has_spaces && has_tabs {
                    return Err(format!(
                        "Lexer Error (line {}, col {}): Mixed spaces and tabs in indentation are not allowed.",
                        self.line, self.col
                    ));
                }

                // Look ahead for blank/comment lines
                self.skip_whitespace_on_line();
                let next_ch = self.peek();
                if next_ch.is_none() || next_ch == Some('\r') || next_ch == Some('\n') || next_ch == Some('#') || (next_ch == Some('/') && self.peek_next() == Some('/')) {
                    if let Some('#') = self.peek() {
                        while let Some(ch) = self.peek() {
                            if ch == '\n' || ch == '\r' {
                                break;
                            }
                            self.advance();
                        }
                    } else if next_ch == Some('/') && self.peek_next() == Some('/') {
                        self.advance();
                        self.advance();
                        while let Some(ch) = self.peek() {
                            if ch == '\n' || ch == '\r' {
                                break;
                            }
                            self.advance();
                        }
                    }
                    if let Some('\r') = self.peek() {
                        self.advance();
                    }
                    if let Some('\n') = self.peek() {
                        self.advance();
                    }
                    continue;
                }

                // Normal indented line
                let current_indent = *self.indent_stack.last().unwrap();
                if indent_width > current_indent {
                    self.indent_stack.push(indent_width);
                    let ind_tok = self.make_spanned(Token::Indent);
                    self.pending_tokens.push_back(ind_tok);
                } else if indent_width < current_indent {
                    while let Some(&top) = self.indent_stack.last() {
                        if indent_width < top {
                            self.indent_stack.pop();
                            let ded_tok = self.make_spanned(Token::Dedent);
                            self.pending_tokens.push_back(ded_tok);
                        } else {
                            break;
                        }
                    }
                    if self.indent_stack.last() != Some(&indent_width) {
                        return Err(format!(
                            "Lexer Error (line {}, col {}): Indentation level ({} spaces) does not match any outer indentation level.",
                            self.line, self.col, indent_width
                        ));
                    }
                }
                self.is_at_line_start = false;
                continue;
            }

            self.skip_whitespace_on_line();

            if self.pos >= self.source.len() {
                continue;
            }

            let ch = self.peek().unwrap();

            if ch == '#' || (ch == '/' && self.peek_next() == Some('/')) {
                while let Some(c) = self.peek() {
                    if c == '\n' || c == '\r' {
                        break;
                    }
                    self.advance();
                }
                continue;
            }

            if ch == '\r' || ch == '\n' {
                if ch == '\r' {
                    self.advance();
                }
                if self.peek() == Some('\n') {
                    self.advance();
                }
                
                if self.paren_nesting == 0 {
                    self.is_at_line_start = true;
                    return Ok(self.make_spanned(Token::Newline));
                } else {
                    continue;
                }
            }

            let start_line = self.line;
            let start_col = self.col;

            if ch == '"' {
                self.advance();
                let mut s = String::new();
                let mut escaped = false;

                loop {
                    match self.advance() {
                        Some('"') if !escaped => break,
                        Some('\\') if !escaped => escaped = true,
                        Some(c) => {
                            if escaped {
                                match c {
                                    'n' => s.push('\n'),
                                    't' => s.push('\t'),
                                    'r' => s.push('\r'),
                                    '\\' => s.push('\\'),
                                    '"' => s.push('"'),
                                    _ => s.push(c),
                                }
                                escaped = false;
                            } else {
                                s.push(c);
                            }
                        }
                        None => {
                            return Err(format!(
                                "Lexer Error (line {}, col {}): Unterminated string literal starting at line {}, col {}.",
                                self.line, self.col, start_line, start_col
                            ));
                        }
                    }
                }
                return Ok(SpannedToken {
                    token: Token::StringLiteral(s),
                    line: start_line,
                    col: start_col,
                });
            }

            if ch.is_ascii_digit() {
                let mut num_str = String::new();
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        num_str.push(self.advance().unwrap());
                    } else {
                        break;
                    }
                }

                if self.peek() == Some('.') && self.peek_next().map_or(false, |c| c.is_ascii_digit()) {
                    num_str.push(self.advance().unwrap());
                    while let Some(c) = self.peek() {
                        if c.is_ascii_digit() {
                            num_str.push(self.advance().unwrap());
                        } else {
                            break;
                        }
                    }
                    let val: f64 = num_str.parse().unwrap();
                    return Ok(SpannedToken {
                        token: Token::FloatLiteral(val),
                        line: start_line,
                        col: start_col,
                    });
                } else {
                    let val: i64 = num_str.parse().unwrap();
                    return Ok(SpannedToken {
                        token: Token::IntLiteral(val),
                        line: start_line,
                        col: start_col,
                    });
                }
            }

            if ch.is_ascii_alphabetic() || ch == '_' {
                let mut id = String::new();
                while let Some(c) = self.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        id.push(self.advance().unwrap());
                    } else {
                        break;
                    }
                }

                let tok = match id.as_str() {
                    "fn" => Token::Fn,
                    "let" => Token::Let,
                    "const" => Token::Const,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "elif" => Token::Elif,
                    "while" => Token::While,
                    "for" => Token::For,
                    "in" => Token::In,
                    "class" => Token::Class,
                    "pyimport" => Token::PyImport,
                    "return" => Token::Return,
                    "true" => Token::True,
                    "false" => Token::False,
                    "try" => Token::Try,
                    "catch" => Token::Catch,
                    "finally" => Token::Finally,
                    "raise" => Token::Raise,
                    "trait" => Token::Trait,
                    "super" => Token::Super,
                    "match" => Token::Match,
                    "case" => Token::Case,
                    "_" => Token::Underscore,
                    "async" => Token::Async,
                    "await" => Token::Await,
                    "not" => Token::Not,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "null" => Token::Null,
                    "optional" => Token::Optional,
                    "and" => Token::And,
                    "or" => Token::Or,
                    "import" => Token::Import,
                    "from" => Token::From,
                    "int" => Token::IntType,
                    "float" => Token::FloatType,
                    "float32" => Token::Float32Type,
                    "string" => Token::StringType,
                    "bool" => Token::BoolType,
                    "void" => Token::VoidType,
                    _ => Token::Identifier(id),
                };
                return Ok(SpannedToken {
                    token: tok,
                    line: start_line,
                    col: start_col,
                });
            }

            self.advance();

            let tok = match ch {
                '+' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::PlusEq
                    } else {
                        Token::Plus
                    }
                }
                '-' => {
                    if self.peek() == Some('>') {
                        self.advance();
                        Token::Arrow
                    } else if self.peek() == Some('=') {
                        self.advance();
                        Token::MinusEq
                    } else {
                        Token::Minus
                    }
                }
                '*' => {
                    if self.peek() == Some('*') {
                        self.advance();
                        Token::Pow
                    } else if self.peek() == Some('=') {
                        self.advance();
                        Token::StarEq
                    } else {
                        Token::Star
                    }
                },
                '/' => Token::Slash,
                '%' => Token::Percent,
                '@' => Token::At,
                '=' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::Eq
                    } else {
                        Token::Assign
                    }
                }
                '!' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::Ne
                    } else {
                        Token::Bang
                    }
                }
                '?' => {
                    if self.peek() == Some('.') {
                        self.advance();
                        Token::QuestionDot
                    } else {
                        Token::Question
                    }
                }
                '<' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::Le
                    } else {
                        Token::Lt
                    }
                }
                '>' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::Ge
                    } else {
                        Token::Gt
                    }
                }
                ':' => Token::Colon,
                ',' => Token::Comma,
                '.' => Token::Dot,
                '[' => {
                    self.paren_nesting += 1;
                    Token::OpenBracket
                }
                ']' => {
                    if self.paren_nesting > 0 {
                        self.paren_nesting -= 1;
                    }
                    Token::CloseBracket
                }
                '(' => {
                    self.paren_nesting += 1;
                    Token::OpenParen
                }
                ')' => {
                    if self.paren_nesting > 0 {
                        self.paren_nesting -= 1;
                    }
                    Token::CloseParen
                }
                '{' => {
                    self.paren_nesting += 1;
                    Token::OpenBrace
                }
                '}' => {
                    if self.paren_nesting > 0 {
                        self.paren_nesting -= 1;
                    }
                    Token::CloseBrace
                }
                '|' => {
                    if self.peek() == Some('>') {
                        self.advance();
                        Token::Pipe
                    } else {
                        return Err(format!(
                            "Lexer Error (line {}, col {}): Unexpected character '|'. Did you mean '|>'?",
                            self.line, self.col
                        ));
                    }
                }
                _ => {
                    return Err(format!(
                        "Lexer Error (line {}, col {}): Unexpected character '{}'.",
                        self.line, self.col, ch
                    ));
                }
            };
            return Ok(SpannedToken {
                token: tok,
                line: start_line,
                col: start_col,
            });
        }
    }

    pub fn tokenize_all(&mut self) -> Result<Vec<SpannedToken>, String> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok.token == Token::EOF;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_lexer() {
        let mut lex = Lexer::new("let x = 42\nfn main():\n    print(\"Hello\")");
        let tokens = lex.tokenize_all().unwrap();
        let tok_types: Vec<Token> = tokens.into_iter().map(|st| st.token).collect();
        assert_eq!(
            tok_types,
            vec![
                Token::Let,
                Token::Identifier("x".into()),
                Token::Assign,
                Token::IntLiteral(42),
                Token::Newline,
                Token::Fn,
                Token::Identifier("main".into()),
                Token::OpenParen,
                Token::CloseParen,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Identifier("print".into()),
                Token::OpenParen,
                Token::StringLiteral("Hello".into()),
                Token::CloseParen,
                Token::Dedent,
                Token::EOF,
            ]
        );
    }

    #[test]
    fn test_indent_dedent_handling() {
        let mut lex = Lexer::new("if true:\n    if false:\n        let x = 1\n    let y = 2");
        let tokens = lex.tokenize_all().unwrap();
        let tok_types: Vec<Token> = tokens.into_iter().map(|st| st.token).collect();
        assert_eq!(
            tok_types,
            vec![
                Token::If,
                Token::True,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::If,
                Token::False,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Let,
                Token::Identifier("x".into()),
                Token::Assign,
                Token::IntLiteral(1),
                Token::Newline,
                Token::Dedent,
                Token::Let,
                Token::Identifier("y".into()),
                Token::Assign,
                Token::IntLiteral(2),
                Token::Dedent,
                Token::EOF,
            ]
        );
    }

    #[test]
    fn test_escapes_and_strings() {
        let mut lex = Lexer::new("\"hello \\\"world\\\" \\n \\\\\"");
        let tokens = lex.tokenize_all().unwrap();
        let tok_types: Vec<Token> = tokens.into_iter().map(|st| st.token).collect();
        assert_eq!(
            tok_types,
            vec![
                Token::StringLiteral("hello \"world\" \n \\".into()),
                Token::EOF,
            ]
        );
    }

    #[test]
    fn test_keywords_and_types() {
        let mut lex = Lexer::new("async await trait super match case _ int float string bool void");
        let tokens = lex.tokenize_all().unwrap();
        let tok_types: Vec<Token> = tokens.into_iter().map(|st| st.token).collect();
        assert_eq!(
            tok_types,
            vec![
                Token::Async,
                Token::Await,
                Token::Trait,
                Token::Super,
                Token::Match,
                Token::Case,
                Token::Underscore,
                Token::IntType,
                Token::FloatType,
                Token::StringType,
                Token::BoolType,
                Token::VoidType,
                Token::EOF,
            ]
        );
    }

    #[test]
    fn test_operators_and_arrows() {
        let mut lex = Lexer::new("-> |> == != <= >= + - * / %");
        let tokens = lex.tokenize_all().unwrap();
        let tok_types: Vec<Token> = tokens.into_iter().map(|st| st.token).collect();
        assert_eq!(
            tok_types,
            vec![
                Token::Arrow,
                Token::Pipe,
                Token::Eq,
                Token::Ne,
                Token::Le,
                Token::Ge,
                Token::Plus,
                Token::Minus,
                Token::Star,
                Token::Slash,
                Token::Percent,
                Token::EOF,
            ]
        );
    }

    #[test]
    fn test_paren_nesting() {
        // While inside parens/brackets/braces, newlines should not generate Newline/Indent/Dedent tokens
        let mut lex = Lexer::new("[\n  1,\n  2\n]");
        let tokens = lex.tokenize_all().unwrap();
        let tok_types: Vec<Token> = tokens.into_iter().map(|st| st.token).collect();
        assert_eq!(
            tok_types,
            vec![
                Token::OpenBracket,
                Token::IntLiteral(1),
                Token::Comma,
                Token::IntLiteral(2),
                Token::CloseBracket,
                Token::EOF,
            ]
        );
    }
}
