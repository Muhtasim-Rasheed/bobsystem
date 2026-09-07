use crate::{Spanned, lexer::Token};
use std::fmt;

#[derive(Debug, Clone)]
pub enum Operand {
    Imm(i64),
    Label(String),
}

#[derive(Debug, Clone)]
pub enum ParsedInstr {
    Nop,
    AddRR {
        dst: u8,
        src1: u8,
        src2: u8,
    },
    AddRI {
        dst: u8,
        src1: u8,
        imm: i64,
    },
    AddIpR {
        dst: u8,
        src: u8,
    },
    AddIpI {
        dst: u8,
        imm: i64,
    },
    AndRR {
        dst: u8,
        src1: u8,
        src2: u8,
    },
    AndRI {
        dst: u8,
        src1: u8,
        imm: i64,
    },
    AndIpR {
        dst: u8,
        src: u8,
    },
    AndIpI {
        dst: u8,
        imm: i64,
    },
    NotR {
        dst: u8,
        src: u8,
    },
    NotIp {
        dst: u8,
    },
    Ld {
        dst: u8,
        target: Operand,
    },
    Ldi {
        dst: u8,
        target: Operand,
    },
    Ldr {
        dst: u8,
        base: u8,
        imm: i64,
    },
    St {
        src: u8,
        target: Operand,
    },
    Sti {
        src: u8,
        target: Operand,
    },
    Str {
        src: u8,
        base: u8,
        imm: i64,
    },
    Br {
        n: bool,
        z: bool,
        p: bool,
        target: Operand,
    },
    Jmp {
        src: u8,
    },
    Jsr {
        target: Operand,
    },
    JsrR {
        src: u8,
    },
    Lea {
        dst: u8,
        target: Operand,
    },
    Ret,
    Trap {
        vector: u8,
    },
}

#[derive(Debug, Clone)]
pub enum LineContent {
    Instr(ParsedInstr),
    FillDirective(Operand),
    StringDirective(String),
    GlobalDirective(String),
}

impl LineContent {
    pub fn word_count(&self) -> usize {
        match self {
            LineContent::Instr(_) => 1,
            LineContent::FillDirective(_) => 1,
            LineContent::StringDirective(s) => s.chars().count() + 1,
            LineContent::GlobalDirective(_) => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceLine {
    pub label: Option<String>,
    pub content: Option<LineContent>,
    pub line: usize,
}

#[derive(Debug)]
pub enum ParseErrorKind {
    UnexpectedEof,
    ExpectedRegister,
    ExpectedOperand,
    ExpectedImmediate,
    LabelNotAllowedHere,
    UnknownMnemonic(String),
    UnknownDirective(String),
    WrongOperandCount { mnemonic: String, got: usize },
    TrailingTokens,
}

#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub kind: ParseErrorKind,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ParseErrorKind::*;
        write!(f, "line {}: ", self.line)?;
        match &self.kind {
            UnexpectedEof => write!(f, "unexpected end of input"),
            ExpectedRegister => write!(f, "expected a register (r0-r7)"),
            ExpectedOperand => write!(f, "expected an operand"),
            ExpectedImmediate => write!(f, "expected an immediate value, found a label"),
            LabelNotAllowedHere => write!(
                f,
                "a label is not valid here, expected a register or immediate"
            ),
            UnknownMnemonic(m) => write!(f, "unknown mnemonic '{m}'"),
            UnknownDirective(d) => write!(f, "unknown directive '.{d}'"),
            WrongOperandCount { mnemonic, got } => {
                write!(f, "wrong number of operands for '{mnemonic}' (got {got})")
            }
            TrailingTokens => write!(f, "unexpected extra tokens at end of line"),
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse(tokens: &[Spanned<Token>]) -> Result<Vec<SourceLine>, ParseError> {
    let mut lines = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        if tokens[i].value == Token::Newline {
            i += 1;
            continue;
        }

        let line_no = tokens[i].line;
        let mut label = None;

        if let Token::Word(name) = &tokens[i].value {
            if tokens.get(i + 1).map(|t| &t.value) == Some(&Token::Colon) {
                label = Some(name.clone());
                i += 2;
            }
        }

        let start = i;
        while i < tokens.len() && tokens[i].value != Token::Newline {
            i += 1;
        }
        let line_tokens = &tokens[start..i];
        if i < tokens.len() {
            i += 1;
        }

        let content = if line_tokens.is_empty() {
            None
        } else {
            Some(parse_content(line_tokens, line_no)?)
        };

        lines.push(SourceLine {
            label,
            content,
            line: line_no,
        });
    }

    Ok(lines)
}

fn parse_content(tokens: &[Spanned<Token>], line: usize) -> Result<LineContent, ParseError> {
    match &tokens[0].value {
        Token::Directive(name) => parse_directive(name, &tokens[1..], line),
        Token::Word(mnemonic) => {
            let instr = parse_instr(mnemonic, &tokens[1..], line)?;
            Ok(LineContent::Instr(instr))
        }
        _ => Err(ParseError {
            line,
            kind: ParseErrorKind::UnexpectedEof,
        }),
    }
}

fn parse_directive(
    name: &str,
    operands: &[Spanned<Token>],
    line: usize,
) -> Result<LineContent, ParseError> {
    let operands = strip_commas(operands);
    match name {
        "fill" => {
            if operands.len() != 1 {
                return Err(ParseError {
                    line,
                    kind: ParseErrorKind::TrailingTokens,
                });
            }
            let op = expect_operand(&operands, 0, line)?;
            Ok(LineContent::FillDirective(op))
        }
        "string" => {
            if operands.len() != 1 {
                return Err(ParseError {
                    line,
                    kind: ParseErrorKind::TrailingTokens,
                });
            }
            match &operands[0].value {
                Token::StringLit(s) => Ok(LineContent::StringDirective(s.clone())),
                _ => Err(ParseError {
                    line,
                    kind: ParseErrorKind::ExpectedOperand,
                }),
            }
        }
        "global" => {
            if operands.len() != 1 {
                return Err(ParseError {
                    line,
                    kind: ParseErrorKind::TrailingTokens,
                });
            }
            match &operands[0].value {
                Token::Word(name) => Ok(LineContent::GlobalDirective(name.clone())),
                _ => Err(ParseError {
                    line,
                    kind: ParseErrorKind::ExpectedOperand,
                }),
            }
        }
        other => Err(ParseError {
            line,
            kind: ParseErrorKind::UnknownDirective(other.to_string()),
        }),
    }
}

fn strip_commas(tokens: &[Spanned<Token>]) -> Vec<Spanned<Token>> {
    tokens
        .iter()
        .filter(|t| t.value != Token::Comma)
        .cloned()
        .collect()
}

fn expect_operand(
    operands: &[Spanned<Token>],
    idx: usize,
    line: usize,
) -> Result<Operand, ParseError> {
    match operands.get(idx).map(|t| &t.value) {
        Some(Token::Immediate(n)) => Ok(Operand::Imm(*n)),
        Some(Token::Word(name)) => Ok(Operand::Label(name.clone())),
        _ => Err(ParseError {
            line,
            kind: ParseErrorKind::ExpectedOperand,
        }),
    }
}

fn expect_register(operands: &[Spanned<Token>], idx: usize, line: usize) -> Result<u8, ParseError> {
    match operands.get(idx).map(|t| &t.value) {
        Some(Token::Register(r)) => Ok(*r),
        _ => Err(ParseError {
            line,
            kind: ParseErrorKind::ExpectedRegister,
        }),
    }
}

fn expect_imm(operands: &[Spanned<Token>], idx: usize, line: usize) -> Result<i64, ParseError> {
    match operands.get(idx).map(|t| &t.value) {
        Some(Token::Immediate(n)) => Ok(*n),
        Some(Token::Word(_)) => Err(ParseError {
            line,
            kind: ParseErrorKind::ExpectedImmediate,
        }),
        _ => Err(ParseError {
            line,
            kind: ParseErrorKind::ExpectedOperand,
        }),
    }
}

fn wrong_count(mnemonic: &str, got: usize, line: usize) -> ParseError {
    ParseError {
        line,
        kind: ParseErrorKind::WrongOperandCount {
            mnemonic: mnemonic.to_string(),
            got,
        },
    }
}

fn parse_instr(
    mnemonic: &str,
    operands: &[Spanned<Token>],
    line: usize,
) -> Result<ParsedInstr, ParseError> {
    let operands = strip_commas(operands);
    let m = mnemonic.to_lowercase();

    match m.as_str() {
        "nop" => Ok(ParsedInstr::Nop),
        "ret" => Ok(ParsedInstr::Ret),

        "add" | "and" => {
            let is_add = m == "add";
            match operands.len() {
                3 => {
                    let dst = expect_register(&operands, 0, line)?;
                    let src1 = expect_register(&operands, 1, line)?;
                    match &operands[2].value {
                        Token::Register(src2) => Ok(if is_add {
                            ParsedInstr::AddRR {
                                dst,
                                src1,
                                src2: *src2,
                            }
                        } else {
                            ParsedInstr::AndRR {
                                dst,
                                src1,
                                src2: *src2,
                            }
                        }),
                        Token::Immediate(imm) => Ok(if is_add {
                            ParsedInstr::AddRI {
                                dst,
                                src1,
                                imm: *imm,
                            }
                        } else {
                            ParsedInstr::AndRI {
                                dst,
                                src1,
                                imm: *imm,
                            }
                        }),
                        _ => Err(ParseError {
                            line,
                            kind: ParseErrorKind::LabelNotAllowedHere,
                        }),
                    }
                }
                2 => {
                    let dst = expect_register(&operands, 0, line)?;
                    match &operands[1].value {
                        Token::Register(src) => Ok(if is_add {
                            ParsedInstr::AddIpR { dst, src: *src }
                        } else {
                            ParsedInstr::AndIpR { dst, src: *src }
                        }),
                        Token::Immediate(imm) => Ok(if is_add {
                            ParsedInstr::AddIpI { dst, imm: *imm }
                        } else {
                            ParsedInstr::AndIpI { dst, imm: *imm }
                        }),
                        _ => Err(ParseError {
                            line,
                            kind: ParseErrorKind::LabelNotAllowedHere,
                        }),
                    }
                }
                n => Err(wrong_count(mnemonic, n, line)),
            }
        }

        "not" => match operands.len() {
            2 => {
                let dst = expect_register(&operands, 0, line)?;
                let src = expect_register(&operands, 1, line)?;
                Ok(ParsedInstr::NotR { dst, src })
            }
            1 => {
                let dst = expect_register(&operands, 0, line)?;
                Ok(ParsedInstr::NotIp { dst })
            }
            n => Err(wrong_count(mnemonic, n, line)),
        },

        "ld" | "ldi" | "lea" => {
            if operands.len() != 2 {
                return Err(wrong_count(mnemonic, operands.len(), line));
            }
            let dst = expect_register(&operands, 0, line)?;
            let target = expect_operand(&operands, 1, line)?;
            Ok(match m.as_str() {
                "ld" => ParsedInstr::Ld { dst, target },
                "ldi" => ParsedInstr::Ldi { dst, target },
                _ => ParsedInstr::Lea { dst, target },
            })
        }

        "st" | "sti" => {
            if operands.len() != 2 {
                return Err(wrong_count(mnemonic, operands.len(), line));
            }
            let src = expect_register(&operands, 0, line)?;
            let target = expect_operand(&operands, 1, line)?;
            Ok(if m == "st" {
                ParsedInstr::St { src, target }
            } else {
                ParsedInstr::Sti { src, target }
            })
        }

        "ldr" | "str" => {
            if operands.len() != 3 {
                return Err(wrong_count(mnemonic, operands.len(), line));
            }
            let reg = expect_register(&operands, 0, line)?;
            let base = expect_register(&operands, 1, line)?;
            let imm = expect_imm(&operands, 2, line)?;
            Ok(if m == "ldr" {
                ParsedInstr::Ldr {
                    dst: reg,
                    base,
                    imm,
                }
            } else {
                ParsedInstr::Str {
                    src: reg,
                    base,
                    imm,
                }
            })
        }

        "jmp" => {
            if operands.len() != 1 {
                return Err(wrong_count(mnemonic, operands.len(), line));
            }
            Ok(ParsedInstr::Jmp {
                src: expect_register(&operands, 0, line)?,
            })
        }

        "jsr" => {
            if operands.len() != 1 {
                return Err(wrong_count(mnemonic, operands.len(), line));
            }
            match &operands[0].value {
                Token::Register(r) => Ok(ParsedInstr::JsrR { src: *r }),
                Token::Immediate(_) | Token::Word(_) => Ok(ParsedInstr::Jsr {
                    target: expect_operand(&operands, 0, line)?,
                }),
                _ => Err(ParseError {
                    line,
                    kind: ParseErrorKind::ExpectedOperand,
                }),
            }
        }

        "trap" => {
            if operands.len() != 1 {
                return Err(wrong_count(mnemonic, operands.len(), line));
            }
            let v = expect_imm(&operands, 0, line)?;
            Ok(ParsedInstr::Trap { vector: v as u8 })
        }

        _ if m == "br" || (m.starts_with("br") && m[2..].chars().all(|c| "nzp".contains(c))) => {
            if operands.len() != 1 {
                return Err(wrong_count(mnemonic, operands.len(), line));
            }
            let flags = &m[2..];
            let (n, z, p) = if flags.is_empty() {
                (true, true, true)
            } else {
                (
                    flags.contains('n'),
                    flags.contains('z'),
                    flags.contains('p'),
                )
            };
            let target = expect_operand(&operands, 0, line)?;
            Ok(ParsedInstr::Br { n, z, p, target })
        }

        other => Err(ParseError {
            line,
            kind: ParseErrorKind::UnknownMnemonic(other.to_string()),
        }),
    }
}
