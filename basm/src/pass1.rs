use crate::parser::{LineContent, SourceLine};
use bobj::bobject::{Binding, Symbol};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
pub enum SymbolError {
    DuplicateLabel {
        name: String,
        first_line: usize,
        dup_line: usize,
    },
    UndefinedGlobal {
        name: String,
        line: usize,
    },
    ProgramTooLarge {
        words: usize,
    },
    LabelOutOfRange {
        name: String,
        line: usize,
    },
}

impl fmt::Display for SymbolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolError::DuplicateLabel {
                name,
                first_line,
                dup_line,
            } => write!(
                f,
                "line {dup_line}: label '{name}' already defined on line {first_line}"
            ),
            SymbolError::UndefinedGlobal { name, line } => write!(
                f,
                "line {line}: .global '{name}' refers to a label that is never defined"
            ),
            SymbolError::ProgramTooLarge { words } => {
                write!(f, "program is {words} words, but the maximum is 65536")
            }
            SymbolError::LabelOutOfRange { name, line } => write!(
                f,
                "line {line}: label '{name}' sits at word 65536, one past the last valid address"
            ),
        }
    }
}

impl std::error::Error for SymbolError {}

#[derive(Debug)]
pub struct LayoutResult {
    pub symbols: Vec<Symbol>,
    pub offsets: Vec<usize>,
}

pub fn build_symbols(lines: &[SourceLine]) -> Result<LayoutResult, SymbolError> {
    let mut symbols: Vec<Symbol> = Vec::new();
    let mut name_to_index: HashMap<String, usize> = HashMap::new();
    let mut first_def_line: HashMap<String, usize> = HashMap::new();
    let mut offsets = Vec::with_capacity(lines.len());
    let mut offset: usize = 0;

    for line in lines {
        offsets.push(offset);

        if let Some(name) = &line.label {
            if name_to_index.contains_key(name) {
                return Err(SymbolError::DuplicateLabel {
                    name: name.clone(),
                    first_line: first_def_line[name],
                    dup_line: line.line,
                });
            }
            let offset_u16: u16 = offset
                .try_into()
                .map_err(|_| SymbolError::LabelOutOfRange {
                    name: name.clone(),
                    line: line.line,
                })?;
            name_to_index.insert(name.clone(), symbols.len());
            first_def_line.insert(name.clone(), line.line);
            symbols.push(Symbol {
                name: name.clone(),
                offset: offset_u16,
                binding: Binding::Local,
            });
        }

        if let Some(content) = &line.content {
            offset += content.word_count();
        }
    }

    if offset > 0x10000 {
        return Err(SymbolError::ProgramTooLarge { words: offset });
    }

    for line in lines {
        if let Some(LineContent::GlobalDirective(name)) = &line.content {
            match name_to_index.get(name) {
                Some(&idx) => symbols[idx].binding = Binding::Global,
                None => {
                    return Err(SymbolError::UndefinedGlobal {
                        name: name.clone(),
                        line: line.line,
                    });
                }
            }
        }
    }

    Ok(LayoutResult { symbols, offsets })
}
