use std::path::PathBuf;

use bobin::bobinary::BobinaryFile;

use crate::machine::Machine;

mod inst;
mod machine;

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
    let Some(executable_path) = args.next().map(PathBuf::from) else {
        err!("Usage: {} <executable>", program_name);
    };
    if !executable_path.exists() {
        err!("Error: {} doesn't exist", executable_path.display());
    }
    if executable_path.is_dir() {
        err!("Error: {} is a directory", executable_path.display());
    }
    let bobinary_raw = std::fs::read(executable_path).expect("couldn't read file");
    let bobinary = match BobinaryFile::from_bytes(&bobinary_raw) {
        Ok(bobin) => bobin,
        Err(e) => err!("Error: broken bobinary file: {}", e),
    };
    let mut memory = vec![0; 0x10000].into_boxed_slice();
    memory[0..bobinary.words.len()].copy_from_slice(&bobinary.words);
    let mut machine = Machine::new(memory);
    machine.run();
}
