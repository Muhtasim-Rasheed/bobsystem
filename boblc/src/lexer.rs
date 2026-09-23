use std::rc::Rc;

use crate::{
    CompilerCtx,
    diag::Diag,
    source::{Span, Spanned},
};

pub type Token = Spanned<TokenKind>;

#[derive(Debug)]
pub enum TokenKind {
    IntLit(u16),
    CharLit(char),
    StrLit(String),
    Ident(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    Colon,
    ColonColon,
    Semicolon,
    Eq,
    Comma,
    EqEq,
    Gt,
    Ge,
    Lt,
    Le,
    Bang,
    BangEq,
    Tilde,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    AmpAmp,
    Pipe,
    PipePipe,
    Caret,
    Eof,
    Error,
}

pub struct Lexer {
    pub ctx: Rc<CompilerCtx>,
    position: usize,
}

impl Lexer {
    pub fn new(ctx: &Rc<CompilerCtx>) -> Self {
        Self {
            ctx: Rc::clone(ctx),
            position: 0,
        }
    }

    fn content(&self) -> &str {
        &self.ctx.source.content
    }

    fn peek(&self) -> Option<char> {
        self.content()[self.position..].chars().next()
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.position += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        if self.content()[self.position..].starts_with("//") {
            self.position += 2;
            while let Some(c) = self.peek() {
                if c == '\n' {
                    break;
                }
                self.position += c.len_utf8();
            }
        } else if self.content()[self.position..].starts_with("/*") {
            self.position += 2;
            let mut depth = 1;
            while let Some(c) = self.peek() {
                if c == '/' && self.content()[self.position + 1..].starts_with('*') {
                    depth += 1;
                    self.position += 2;
                } else if c == '*' && self.content()[self.position + 1..].starts_with('/') {
                    depth -= 1;
                    self.position += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    self.position += c.len_utf8();
                }
            }
            if depth > 0 {
                self.ctx.emit_diag(
                    Diag::new(&self.ctx.source, "undelimited multiline comment")
                        .with_message_label(
                            "expected */",
                            Span::new(self.position, self.position),
                            true,
                        ),
                );
            }
        }
    }

    fn skip_trivial(&mut self) {
        loop {
            self.skip_whitespace();
            if self.content()[self.position..].starts_with("//")
                || self.content()[self.position..].starts_with("/*")
            {
                self.skip_comment();
            } else {
                break;
            }
        }
    }

    fn read_num(&mut self) -> Token {
        let mut num = 0_u16;
        let start = self.position;

        while let Some(d) = self.peek()
            && d.is_ascii_digit()
        {
            self.position += d.len_utf8();
            let d = d as u8 - 48;
            if let Some(n) = num
                .checked_mul(10)
                .and_then(|num| num.checked_add(d as u16))
            {
                num = n;
            } else {
                self.ctx.emit_diag(
                    Diag::new(&self.ctx.source, "unsigned 16 bit integer overflow")
                        .with_message_label(
                            "the number is too big for a unsigned 16 bit integer",
                            Span::new(start, self.position),
                            true,
                        ),
                );
                break;
            }
        }

        Token::new(TokenKind::IntLit(num), Span::new(start, self.position))
    }

    fn read_escape(&mut self) -> char {
        match self.peek() {
            Some('n') => {
                self.position += 1;
                '\n'
            }
            Some('t') => {
                self.position += 1;
                '\t'
            }
            Some('r') => {
                self.position += 1;
                '\r'
            }
            Some('0') => {
                self.position += 1;
                '\0'
            }
            Some('\\') => {
                self.position += 1;
                '\\'
            }
            Some('\'') => {
                self.position += 1;
                '\''
            }
            Some('"') => {
                self.position += 1;
                '"'
            }
            Some(other) => {
                let start = self.position - 1; // include the backslash
                self.position += other.len_utf8();
                self.ctx.emit_diag(
                    Diag::new(
                        &self.ctx.source,
                        format!("unknown escape sequence `\\{other}`"),
                    )
                    .with_label(Span::new(start, self.position), true),
                );
                other
            }
            None => {
                self.ctx.emit_diag(
                    Diag::new(&self.ctx.source, "unterminated escape sequence at EOF")
                        .with_label(Span::new(self.position, self.position), true),
                );
                '\\'
            }
        }
    }

    fn read_char(&mut self) -> Token {
        let start = self.position;
        self.position += 1;

        let ch = match self.peek() {
            Some('\'') => {
                self.position += 1;
                self.ctx.emit_diag(
                    Diag::new(&self.ctx.source, "empty character literal")
                        .with_label(Span::new(start, self.position), true),
                );
                return Token::new(TokenKind::CharLit('\0'), Span::new(start, self.position));
            }
            Some('\\') => {
                self.position += 1;
                self.read_escape()
            }
            Some('\n') | None => {
                self.ctx.emit_diag(
                    Diag::new(&self.ctx.source, "unterminated character literal")
                        .with_label(Span::new(start, self.position), true),
                );
                return Token::new(TokenKind::CharLit('\0'), Span::new(start, self.position));
            }
            Some(c) => {
                self.position += c.len_utf8();
                c
            }
        };

        match self.peek() {
            Some('\'') => {
                self.position += 1;
            }
            Some('\n') | None => {
                self.ctx.emit_diag(
                    Diag::new(&self.ctx.source, "unterminated character literal")
                        .with_message_label(
                            "expected closing '",
                            Span::new(start, self.position),
                            true,
                        ),
                );
            }
            Some(_) => {
                while let Some(c) = self.peek() {
                    if c == '\'' || c == '\n' {
                        break;
                    }
                    self.position += c.len_utf8();
                }
                if self.peek() == Some('\'') {
                    self.position += 1;
                }
                self.ctx.emit_diag(
                    Diag::new(
                        &self.ctx.source,
                        "character literal contains more than one character",
                    )
                    .with_label(Span::new(start, self.position), true)
                    .with_text_help(
                        "if you wanted to write a string, wrap the literal in \" instead of '",
                    ),
                );
            }
        }

        Token::new(TokenKind::CharLit(ch), Span::new(start, self.position))
    }

    fn read_str(&mut self) -> Token {
        let start = self.position;
        self.position += 1;
        let mut s = String::new();

        loop {
            match self.peek() {
                Some('"') => {
                    self.position += 1;
                    break;
                }
                Some('\n') | None => {
                    self.ctx.emit_diag(
                        Diag::new(&self.ctx.source, "unterminated string literal")
                            .with_label(Span::new(start, self.position), true),
                    );
                    break;
                }
                Some('\\') => {
                    self.position += 1;
                    s.push(self.read_escape());
                }
                Some(c) => {
                    self.position += c.len_utf8();
                    s.push(c);
                }
            }
        }

        Token::new(TokenKind::StrLit(s), Span::new(start, self.position))
    }

    pub fn read_ident(&mut self) -> Token {
        let start = self.position;
        let mut ident = String::new();

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.position += c.len_utf8();
                ident.push(c);
            } else {
                break;
            }
        }

        Token::new(TokenKind::Ident(ident), Span::new(start, self.position))
    }

    pub fn read_symbol(&mut self) -> Token {
        let start = self.position;
        let c = self.peek().unwrap();
        self.position += c.len_utf8();
        let tk = match c {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            ':' if self.peek() == Some(':') => {
                self.position += 1;
                TokenKind::ColonColon
            }
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            '=' if self.peek() == Some('=') => {
                self.position += 1;
                TokenKind::EqEq
            }
            '=' => TokenKind::Eq,
            '>' if self.peek() == Some('=') => {
                self.position += 1;
                TokenKind::Ge
            }
            '>' => TokenKind::Gt,
            '<' if self.peek() == Some('=') => {
                self.position += 1;
                TokenKind::Le
            }
            '<' => TokenKind::Lt,
            '!' if self.peek() == Some('=') => {
                self.position += 1;
                TokenKind::BangEq
            }
            '!' => TokenKind::Bang,
            '~' => TokenKind::Tilde,
            ',' => TokenKind::Comma,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '&' if self.peek() == Some('&') => {
                self.position += 1;
                TokenKind::AmpAmp
            }
            '&' => TokenKind::Amp,
            '|' if self.peek() == Some('|') => {
                self.position += 1;
                TokenKind::PipePipe
            }
            '|' => TokenKind::Pipe,
            '^' => TokenKind::Caret,
            _ => {
                self.ctx.emit_diag(
                    Diag::new(&self.ctx.source, "unexpected symbol").with_message_label(
                        format!("unexpected symbol '{c}'"),
                        Span::new(start, self.position),
                        true,
                    ),
                );
                TokenKind::Error
            }
        };
        Token::new(tk, Span::new(start, self.position))
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_trivial();

        if self.position >= self.content().len() {
            return Token::new(TokenKind::Eof, Span::new(self.position, self.position));
        }

        let Some(c) = self.peek() else {
            return Token::new(TokenKind::Eof, Span::new(self.position, self.position));
        };

        if c.is_ascii_digit() {
            self.read_num()
        } else if c == '\'' {
            self.read_char()
        } else if c == '"' {
            self.read_str()
        } else if c.is_alphabetic() || c == '_' {
            self.read_ident()
        } else {
            self.read_symbol()
        }
    }
}
