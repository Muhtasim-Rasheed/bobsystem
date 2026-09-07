use crate::Spanned;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    Directive(String),
    Register(u8),
    Immediate(i64),
    StringLit(String),
    Colon,
    Comma,
    Newline,
}

#[derive(Debug)]
pub enum LexError {
    UnterminatedString { line: usize },
    InvalidNumber { text: String, line: usize },
    UnexpectedChar { ch: char, line: usize },
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexError::UnterminatedString { line } => {
                write!(f, "line {line}: unterminated string literal")
            }
            LexError::InvalidNumber { text, line } => {
                write!(f, "line {line}: invalid numeric literal '{text}'")
            }
            LexError::UnexpectedChar { ch, line } => {
                write!(f, "line {line}: unexpected character '{ch}'")
            }
        }
    }
}

impl std::error::Error for LexError {}

pub fn lex(src: &str) -> Result<Vec<Spanned<Token>>, LexError> {
    let mut tokens = Vec::new();
    let mut chars = src.chars().peekable();
    let mut line = 1usize;

    macro_rules! push {
        ($tok:expr) => {
            tokens.push(Spanned { value: $tok, line })
        };
    }

    while let Some(&ch) = chars.peek() {
        match ch {
            '\n' => {
                push!(Token::Newline);
                line += 1;
                chars.next();
            }
            c if c.is_whitespace() => {
                chars.next();
            }
            ';' => {
                while let Some(&c) = chars.peek() {
                    if c == '\n' {
                        break;
                    }
                    chars.next();
                }
            }
            ',' => {
                push!(Token::Comma);
                chars.next();
            }
            ':' => {
                push!(Token::Colon);
                chars.next();
            }
            '"' => {
                chars.next(); // consume opening quote
                let mut s = String::new();
                loop {
                    match chars.next() {
                        None | Some('\n') => return Err(LexError::UnterminatedString { line }),
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some('\\') => s.push('\\'),
                            Some('"') => s.push('"'),
                            Some(other) => s.push(other),
                            None => return Err(LexError::UnterminatedString { line }),
                        },
                        Some(c) => s.push(c),
                    }
                }
                push!(Token::StringLit(s));
            }
            '.' => {
                chars.next();
                let mut name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                push!(Token::Directive(name.to_lowercase()));
            }
            c if c.is_ascii_digit() || (c == '-' && peek_is_digit(&mut chars)) => {
                let mut text = String::new();
                if c == '-' {
                    text.push('-');
                    chars.next();
                }
                let mut is_hex = false;
                if chars.peek() == Some(&'0') {
                    text.push('0');
                    chars.next();
                    if matches!(chars.peek(), Some('x') | Some('X')) {
                        is_hex = true;
                        text.push(chars.next().unwrap());
                    }
                }
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_hexdigit() || c == '_' {
                        if c != '_' {
                            text.push(c);
                        }
                        chars.next();
                    } else {
                        break;
                    }
                }

                let parsed = if is_hex {
                    let digits = text.trim_start_matches('-').trim_start_matches("0x");
                    i64::from_str_radix(digits, 16).ok()
                } else {
                    text.parse::<i64>().ok()
                };
                let Some(mut value) = parsed else {
                    return Err(LexError::InvalidNumber { text, line });
                };
                if is_hex && text.starts_with('-') {
                    value = -value;
                }
                push!(Token::Immediate(value));
            }
            c if c.is_alphabetic() || c == '_' => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        word.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Some(reg) = parse_register(&word) {
                    push!(Token::Register(reg));
                } else {
                    push!(Token::Word(word));
                }
            }
            other => return Err(LexError::UnexpectedChar { ch: other, line }),
        }
    }

    Ok(tokens)
}

fn peek_is_digit(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();
    lookahead.next();
    matches!(lookahead.peek(), Some(c) if c.is_ascii_digit())
}

fn parse_register(word: &str) -> Option<u8> {
    if word.len() != 2 {
        return None;
    }
    let mut chars = word.chars();
    let r = chars.next()?;
    if r != 'r' && r != 'R' {
        return None;
    }
    let digit = chars.next()?;
    digit.to_digit(10).filter(|&d| d <= 7).map(|d| d as u8)
}
