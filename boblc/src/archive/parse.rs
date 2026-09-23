use crate::lex::Token;

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken {
        expected: Option<&'static str>,
        got: Token,
    },
    UnexpectedEOF {
        expected: &'static str,
        last_line: usize,
    },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedToken { expected, got } => match expected {
                Some(expected) => write!(
                    f,
                    "line {}: expected {expected} but got `{}`",
                    got.line, got.val
                ),
                None => write!(f, "line {}: unexpected `{}`", got.line, got.val),
            },
            ParseError::UnexpectedEOF {
                expected,
                last_line,
            } => write!(f, "line {}: expected {} but got EOF", last_line, expected),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug)]
pub struct Program {
    pub includes: Vec<String>,
    pub functions: Vec<FunctionDecl>,
}

#[derive(Debug)]
pub enum FunctionDecl {
    External { name: String },
    Defined { name: String, body: Vec<Word> },
}

#[derive(Clone)]
pub enum Word {
    PushInt(i64),
    PushString(String),
    Drop,
    Add,
    AddInt(i64),
    AddStr(String),
    And,
    AndInt(i64),
    AndStr(String),
    Not,
    NotInt(i64),
    NotStr(String),
    Dup,
    TwoDup,
    Swap,
    Rot,
    Dr,
    Rd,
    While,
    If(usize),          // target when condition is false
    Else(usize),        // target after the else branch
    Do(usize),          // target when condition is false
    End(Option<usize>), // Some(while index) for loops, None for if
    Trap1,
    Trap2,
    Call(String),
}

impl std::fmt::Debug for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        macro_rules! write_str_lit {
            ($s:expr, $after:expr) => {
                write!(
                    f,
                    "\"{}\"{}",
                    $s.replace("\n", "\\n")
                        .replace("\t", "\\t")
                        .replace("\\", "\\\\")
                        .replace("\"", "\\\""),
                    $after
                )
            };
        }
        use Word::*;
        match self {
            PushInt(v) => write!(f, "{v}"),
            PushString(s) => write_str_lit!(s, ""),
            Drop => write!(f, "drop"),
            Add => write!(f, "add"),
            AddInt(v) => write!(f, "{v} add"),
            AddStr(s) => write_str_lit!(s, " add"),
            And => write!(f, "and"),
            AndInt(v) => write!(f, "{v} and"),
            AndStr(s) => write_str_lit!(s, " and"),
            Not => write!(f, "not"),
            NotInt(v) => write!(f, "{v} not"),
            NotStr(s) => write_str_lit!(s, " not"),
            Dup => write!(f, "dup"),
            TwoDup => write!(f, "2dup"),
            Swap => write!(f, "swap"),
            Rot => write!(f, "rot"),
            Dr => write!(f, "dr"),
            Rd => write!(f, "rd"),
            While => write!(f, "while"),
            If(_) => write!(f, "if"),
            Else(_) => write!(f, "else"),
            Do(_) => write!(f, "do"),
            End(_) => write!(f, "end"),
            Trap1 => write!(f, "trap1"),
            Trap2 => write!(f, "trap2"),
            Call(func) => write!(f, "{func}"),
        }
    }
}

pub fn parse(tokens: &[Token]) -> Result<Program, ParseError> {
    let mut pos = 0;
    let mut program = Program {
        includes: Vec::new(),
        functions: Vec::new(),
    };

    while pos < tokens.len() {
        match tokens[pos].val.as_str() {
            "include" => program.includes.push(parse_include(tokens, &mut pos)?),
            "func" => program.functions.push(parse_func(tokens, &mut pos)?),
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: Some("func or include"),
                    got: tokens[pos].clone(),
                });
            }
        }
    }

    Ok(program)
}

fn parse_include(tokens: &[Token], pos: &mut usize) -> Result<String, ParseError> {
    eat(tokens, "include", pos)?;
    eat_string(tokens, pos)
}

fn parse_func(tokens: &[Token], pos: &mut usize) -> Result<FunctionDecl, ParseError> {
    eat(tokens, "func", pos)?;
    if matches(tokens, "ext", pos) {
        Ok(FunctionDecl::External {
            name: next(tokens, pos)?,
        })
    } else {
        let name = next(tokens, pos)?;
        let mut body = Vec::new();
        let mut s = 1;
        loop {
            let t = next(tokens, pos).map_err(|_| ParseError::UnexpectedEOF {
                expected: "end",
                last_line: tokens.last().map(|v| v.line).unwrap_or(0),
            })?;

            if t == "end" {
                s -= 1;
                if s == 0 {
                    break;
                }
            }
            if t == "if" || t == "do" {
                s += 1;
            }
            body.push(token2word(&t));
        }
        Ok(FunctionDecl::Defined { name, body })
    }
}

fn token2word(token: &str) -> Word {
    match token {
        "drop" => Word::Drop,
        "add" => Word::Add,
        "and" => Word::And,
        "not" => Word::Not,
        "dup" => Word::Dup,
        "2dup" => Word::TwoDup,
        "swap" => Word::Swap,
        "rot" => Word::Rot,
        "dr" => Word::Dr,
        "rd" => Word::Rd,
        "while" => Word::While,
        "if" => Word::If(0),      // filled in later
        "else" => Word::Else(0),  // filled in later
        "do" => Word::Do(0),      // filled in later
        "end" => Word::End(None), // filled in later
        "trap1" => Word::Trap1,
        "trap2" => Word::Trap2,
        s if is_push_string_token(s) => Word::PushString(s[1..s.len() - 1].into()),
        s if let Ok(num) = s.parse::<i64>() => Word::PushInt(num),
        s => Word::Call(s.into()),
    }
}

fn is_push_string_token(token: &str) -> bool {
    token.starts_with('"') && token.ends_with('"')
}

fn eat(tokens: &[Token], token: &'static str, pos: &mut usize) -> Result<(), ParseError> {
    match tokens.get(*pos) {
        Some(t) if t.val == token => {
            *pos += 1;
            Ok(())
        }
        Some(t) => Err(ParseError::UnexpectedToken {
            expected: Some(token),
            got: t.clone(),
        }),
        None => Err(ParseError::UnexpectedEOF {
            expected: token,
            last_line: tokens.last().map(|v| v.line).unwrap_or(0),
        }),
    }
}

fn eat_string(tokens: &[Token], pos: &mut usize) -> Result<String, ParseError> {
    match tokens.get(*pos) {
        Some(t) if t.val.starts_with('"') && t.val.ends_with('"') => {
            *pos += 1;
            Ok(t.val[1..t.val.len() - 1].into())
        }
        Some(t) => Err(ParseError::UnexpectedToken {
            expected: Some("a string"),
            got: t.clone(),
        }),
        None => Err(ParseError::UnexpectedEOF {
            expected: "a string",
            last_line: tokens.last().map(|v| v.line).unwrap_or(0),
        }),
    }
}

fn next(tokens: &[Token], pos: &mut usize) -> Result<String, ParseError> {
    match tokens.get(*pos) {
        Some(t) => {
            *pos += 1;
            Ok(t.val.clone())
        }
        None => Err(ParseError::UnexpectedEOF {
            expected: "a word",
            last_line: tokens.last().map(|v| v.line).unwrap_or(0),
        }),
    }
}

fn matches(tokens: &[Token], token: &str, pos: &mut usize) -> bool {
    if tokens.get(*pos).is_some_and(|t| t.val == token) {
        *pos += 1;
        true
    } else {
        false
    }
}
