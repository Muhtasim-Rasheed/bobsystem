use std::path::PathBuf;

use basm::{lexer::lex, parser::parse, pass1::build_symbols, pass2::generate_bobject};

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
    let Some(bassembly_path) = args.next().map(PathBuf::from) else {
        err!("Usage: {} <input-bassembly> <output-bobject>", program_name);
    };
    let Some(bobject_path) = args.next().map(PathBuf::from) else {
        err!("Usage: {} <input-bassembly> <output-bobject>", program_name);
    };
    if !bassembly_path.exists() {
        err!("Error: {} doesn't exist", bassembly_path.display());
    }
    if bassembly_path.is_dir() {
        err!("Error: {} is a directory", bassembly_path.display());
    }
    let bassembly_contents = std::fs::read_to_string(bassembly_path).expect("couldn't read file");
    let tokens = match lex(&bassembly_contents) {
        Ok(t) => t,
        Err(e) => err!("Lexing error: {}", e),
    };
    let ast = match parse(&tokens) {
        Ok(t) => t,
        Err(e) => err!("Parsing error: {}", e),
    };
    let layout = match build_symbols(&ast) {
        Ok(t) => t,
        Err(e) => err!("Symbol error: {}", e),
    };
    let bobject = generate_bobject(ast, layout);
    std::fs::write(bobject_path, bobject.to_bytes()).expect("failed to write to output bobject");
}
