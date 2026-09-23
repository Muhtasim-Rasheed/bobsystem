use std::path::PathBuf;

use crate::{
    codegen::generate_bassembly, lex::lex, parse::parse, resolve_includes::resolve_includes,
    resolve_links::resolve_links,
};

mod codegen;
mod lex;
mod literalpool;
mod opt;
mod parse;
mod resolve_includes;
mod resolve_links;

macro_rules! err {
    ($fmt:expr $(, $($arg:tt)*)?) => {{
        use std::io::Write;
        let _ = std::io::stdout().write_fmt(format_args!(concat!($fmt, "\n") $(, $($arg)*)?));
        std::process::exit(1);
    }};
}

fn main() {
    let mut args = std::env::args();
    let program_name = args.next().unwrap();
    let Some(source_path) = args.next().map(PathBuf::from) else {
        err!("Usage: {} <input-source> <output-bobject>", program_name);
    };
    let Some(bobject_path) = args.next().map(PathBuf::from) else {
        err!("Usage: {} <input-source> <output-bobject>", program_name);
    };
    if !source_path.exists() {
        err!("Error: {} doesn't exist", source_path.display());
    }
    if source_path.is_dir() {
        err!("Error: {} is a directory", source_path.display());
    }
    let source_contents = std::fs::read_to_string(&source_path).expect("couldn't read file");
    let tokens = match lex(&source_contents) {
        Ok(t) => t,
        Err(e) => err!("Lexing error: {}", e),
    };
    let mut ast = match parse(&tokens) {
        Ok(a) => a,
        Err(e) => err!("Parsing error: {}", e),
    };
    match resolve_includes(&mut ast, source_path.parent().unwrap()) {
        Ok(()) => {}
        Err(e) => err!("Include error: {}", e),
    }
    println!("{ast:#?}");
    println!("=========================================");
    {
        let mut changed = true;
        while changed {
            changed &= opt::arith_imm::optimize(&mut ast);
            println!("{ast:#?}");
            println!("=========================================");
        }
    }
    match resolve_links(&mut ast) {
        Ok(()) => {}
        Err((f, i)) => err!("Parsing error: unmatched `end` at word {} in {}", i + 1, f),
    }
    let basm_contents = generate_bassembly(ast);
    let basm_tokens =
        basm::lexer::lex(&basm_contents).expect("codegen should output correct bassembly");
    let basm_ast =
        basm::parser::parse(&basm_tokens).expect("codegen should output correct bassembly");
    let basm_layout =
        basm::pass1::build_symbols(&basm_ast).expect("codegen should output correct bassembly");
    let bobject = basm::pass2::generate_bobject(basm_ast, basm_layout);
    std::fs::write(bobject_path, bobject.to_bytes()).expect("failed to write to output bobject");
}
