use crate::{
    literalpool::LiteralPool,
    parse::{FunctionDecl, Program, Word},
};

pub fn generate_bassembly(program: Program) -> String {
    let mut out = String::new();
    let mut literalpool = LiteralPool::new();

    macro_rules! outln {
        ($fmt:expr $(, $($arg:tt)*)?) => {{
            use std::fmt::Write;
            writeln!(out, $fmt $(, $($arg)*)?).unwrap()
        }};
    }

    for function in program.functions {
        if let FunctionDecl::Defined { name, body } = function {
            let mut insts = 0;

            outln!(".global {name}");
            outln!("{name}:");

            // function prologue
            outln!("\tstr r7, r4, 0");
            outln!("\tadd r4, 1");
            insts += 2;

            // function body
            let body_len = body.len();
            for (i, word) in body.into_iter().enumerate() {
                outln!("__{name}_w{i}:                 ;;; {:?} ;;;", word);
                match word {
                    Word::PushInt(i) => {
                        let label = literalpool.intern_int(i);
                        outln!("\tld r0, {label}");
                        outln!("\tstr r0, r6, 0");
                        outln!("\tadd r6, 1");
                        insts += 3;
                    }
                    Word::PushString(s) => {
                        let label = literalpool.intern_string(&s);
                        outln!("\tlea r0, {label}");
                        outln!("\tstr r0, r6, 0");
                        outln!("\tadd r6, 1");
                        insts += 3;
                    }
                    Word::Drop => {
                        outln!("\tadd r6, -1");
                        insts += 1;
                    }
                    Word::Add => {
                        outln!("\tldr r0, r6, -1");
                        outln!("\tldr r1, r6, -2");
                        outln!("\tadd r0, r1");
                        outln!("\tstr r0, r6, -2");
                        outln!("\tadd r6, -1");
                        insts += 5;
                    }
                    Word::AddInt(i) => {
                        outln!("\tldr r0, r6, -1");
                        outln!("\tadd r0, {i}");
                        outln!("\tstr r0, r6, -1");
                        insts += 3;
                    }
                    Word::AddStr(s) => {
                        outln!("\tldr r0, r6, -1");
                        let label = literalpool.intern_string(&s);
                        outln!("\tlea r1, {label}");
                        outln!("\tadd r0, r1");
                        outln!("\tstr r0, r6, -1");
                        insts += 4;
                    }
                    Word::And => {
                        outln!("\tldr r0, r6, -1");
                        outln!("\tldr r1, r6, -2");
                        outln!("\tand r0, r1");
                        outln!("\tstr r0, r6, -2");
                        outln!("\tadd r6, -1");
                        insts += 5;
                    }
                    Word::AndInt(i) => {
                        outln!("\tldr r0, r6, -1");
                        outln!("\tand r0, {i}");
                        outln!("\tstr r0, r6, -1");
                        insts += 3;
                    }
                    Word::AndStr(s) => {
                        outln!("\tldr r0, r6, -1");
                        let label = literalpool.intern_string(&s);
                        outln!("\tlea r1, {label}");
                        outln!("\tand r0, r1");
                        outln!("\tstr r0, r6, -1");
                        insts += 4;
                    }
                    Word::Not => {
                        outln!("\tldr r0, r6, -1");
                        outln!("\tnot r0");
                        outln!("\tstr r0, r6, -1");
                        insts += 3;
                    }
                    Word::NotInt(i) => {
                        outln!("\tand r0, 0");
                        outln!("\tadd r0, {i}");
                        outln!("\tnot r0");
                        outln!("\tstr r0, r6, -1");
                        insts += 4;
                    }
                    Word::NotStr(s) => {
                        let label = literalpool.intern_string(&s);
                        outln!("\tlea r0, {label}");
                        outln!("\tnot r0");
                        outln!("\tstr r0, r6, -1");
                        insts += 3;
                    }
                    Word::Dup => {
                        outln!("\tldr r0, r6, -1");
                        outln!("\tstr r0, r6, 0");
                        outln!("\tadd r6, 1");
                        insts += 3;
                    }
                    Word::TwoDup => {
                        outln!("\tldr r0, r6, -2");
                        outln!("\tstr r0, r6, 0");
                        outln!("\tldr r0, r6, -1");
                        outln!("\tstr r0, r6, 1");
                        outln!("\tadd r6, 2");
                        insts += 5;
                    }
                    Word::Swap => {
                        outln!("\tldr r0, r6, -1");
                        outln!("\tldr r1, r6, -2");
                        outln!("\tstr r0, r6, -2");
                        outln!("\tstr r1, r6, -1");
                        insts += 4;
                    }
                    Word::Rot => {
                        outln!("\tldr r0, r6, -1");
                        outln!("\tldr r1, r6, -2");
                        outln!("\tldr r2, r6, -3");
                        outln!("\tstr r1, r6, -3");
                        outln!("\tstr r0, r6, -2");
                        outln!("\tstr r2, r6, -1");
                        insts += 6;
                    }
                    Word::Dr => {
                        outln!("\tadd r6, -1");
                        outln!("\tldr r0, r6, 0");
                        outln!("\tstr r0, r4, 0");
                        outln!("\tadd r4, 1");
                        insts += 4;
                    }
                    Word::Rd => {
                        outln!("\tadd r4, -1");
                        outln!("\tldr r0, r4, 0");
                        outln!("\tstr r0, r6, 0");
                        outln!("\tadd r6, 1");
                        insts += 4;
                    }
                    Word::While => {}
                    Word::If(skip) | Word::Do(skip) => {
                        outln!("\tadd r6, -1");
                        outln!("\tldr r0, r6, 0");
                        outln!("\tbrz __{name}_w{skip}");
                        insts += 3;
                    }
                    Word::Else(skip) => {
                        outln!("\tbr __{name}_w{skip}");
                        insts += 1;
                    }
                    Word::End(Some(back)) => {
                        outln!("\tbr __{name}_w{back}");
                        insts += 1;
                    }
                    Word::End(None) => {}
                    Word::Trap1 => {
                        outln!("\tadd r6, -1");
                        outln!("\tldr r0, r6, 0");
                        outln!("\ttrap 1");
                        insts += 3;
                    }
                    Word::Trap2 => {
                        outln!("\tadd r6, -1");
                        outln!("\tldr r0, r6, 0");
                        outln!("\ttrap 2");
                        insts += 3;
                    }
                    Word::Call(func) => {
                        let ptr_label = literalpool.intern_func(&func);
                        outln!("\tld r0, {ptr_label}");
                        outln!("\tjsr r0");
                        insts += 2;
                    }
                }

                if insts + literalpool.size() >= 200 {
                    outln!("br __{name}_w{}", i + 1);
                    insts = 1;
                    literalpool.emit_data_section(&mut out);
                    literalpool.clear();
                }
            }

            outln!("br __{name}_w{}", body_len);

            literalpool.emit_data_section(&mut out);
            literalpool.clear();

            // function epilogue
            outln!("__{name}_w{}:", body_len);
            outln!("\tadd r4, -1");
            outln!("\tldr r7, r4, 0");
            outln!("\tret");
        }
    }

    // println!("{out}");

    out
}
