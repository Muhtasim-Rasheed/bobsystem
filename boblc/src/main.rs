use std::{cell::RefCell, path::PathBuf, rc::Rc};

use crate::{diag::Diag, source::SourceFile};

mod ast;
mod codegen;
mod diag;
mod lexer;
mod loader;
mod op;
mod opt;
mod parser;
mod rast;
mod rlower;
mod source;
mod tac;
mod taclower;
mod ty;

macro_rules! err {
    ($fmt:expr $(, $($arg:tt)*)?) => {{
        use std::io::Write;
        let _ = std::io::stdout().write_fmt(format_args!(concat!($fmt, "\n") $(, $($arg)*)?));
        std::process::exit(1);
    }};
}

pub struct CompilerCtx {
    pub source: Rc<SourceFile>,
    pub diags: Rc<RefCell<Vec<Diag>>>,
}

impl CompilerCtx {
    pub fn emit_diag(&self, diag: Diag) {
        self.diags.borrow_mut().push(diag);
    }

    pub fn other_ctx(&self, source: SourceFile) -> Self {
        Self {
            source: Rc::new(source),
            diags: Rc::clone(&self.diags),
        }
    }
}

fn main() {
    let mut args = std::env::args().peekable();
    let program_name = args.next().unwrap();
    let Some(source_path) = args.next().map(PathBuf::from) else {
        err!("Usage: {} <input-source> <output> <[-S]>", program_name);
    };
    let Some(out_path) = args.next().map(PathBuf::from) else {
        err!("Usage: {} <input-source> <output> <[-S]>", program_name);
    };
    let out_basm = match args.peek() {
        Some(s) if s == "-S" => {
            args.next();
            true
        }
        _ => false,
    };
    if !source_path.exists() {
        err!("Error: {} doesn't exist", source_path.display());
    }
    if source_path.is_dir() {
        err!("Error: {} is a directory", source_path.display());
    }
    let Some(rprogram) = loader::compile(&source_path) else {
        std::process::exit(1);
    };
    let func_names = {
        let mut map = std::collections::HashMap::new();
        for module in &rprogram.modules {
            for func in &module.functions {
                map.insert(func.id, func.name.clone());
            }
        }
        map
    };
    let tacprogram = taclower::Lowerer::new(rprogram).lower();
    let bassembly =
        codegen::Codegen::new(&|id| func_names.get(&id).unwrap().clone()).generate(&tacprogram);
    if !out_basm {
        let bobject_file = {
            let tokens = basm::lexer::lex(&bassembly).unwrap();
            let ast = basm::parser::parse(&tokens).unwrap();
            let layout = basm::pass1::build_symbols(&ast).unwrap();
            basm::pass2::generate_bobject(ast, layout)
        };
        std::fs::write(out_path, bobject_file.to_bytes()).unwrap();
    } else {
        std::fs::write(out_path, bassembly).unwrap();
    }
}
