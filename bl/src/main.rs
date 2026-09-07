use std::{collections::HashMap, path::PathBuf};

use bobin::bobinary::BobinaryFile;
use bobj::bobject::{Binding, BobjectFile, Relocation, RelocationKind};
use clap::Parser;

macro_rules! err {
    ($fmt:expr $(, $($arg:tt)*)?) => {{
        use std::io::Write;
        let _ = std::io::stdout().write_fmt(format_args!(concat!($fmt, "\n") $(, $($arg)*)?));
        std::process::exit(1);
    }}
}

/// The Bob Linker
#[derive(Debug, Parser)]
struct Args {
    /// Output bobinary file
    #[arg(short, long, value_name = "FILE")]
    output: PathBuf,

    /// Any number of input bobject files
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let bobject_files = parse_bobject_files(&args);
    let base_addresses = compute_base_addresses(&bobject_files);
    let global_symbol_table = build_global_symbol_table(&bobject_files, &base_addresses);
    let resolved_relocations =
        resolve_relocations(&bobject_files, &base_addresses, &global_symbol_table);
    let fixup_values = compute_fixup_values(&base_addresses, &resolved_relocations);
    let mut words = bobject_files
        .into_iter()
        .map(|v| v.words)
        .collect::<Vec<_>>();
    patch(&fixup_values, &mut words, &resolved_relocations);
    let words = words.into_iter().flatten().collect::<Vec<_>>();

    let bobinary = BobinaryFile { words };
    std::fs::write(args.output, bobinary.to_bytes()).expect("failed to write to output bobinary");
}

fn parse_bobject_files(args: &Args) -> Vec<BobjectFile> {
    let bobject_file_iter = args
        .inputs
        .iter()
        .map(|p| {
            if !p.exists() {
                err!("Error: {} doesn't exist", p.display());
            }
            if p.is_dir() {
                err!("Error: {} is a directory", p.display());
            }
            std::fs::read(p).expect("couldn't read file")
        })
        .map(|c| BobjectFile::from_bytes(&c));

    let mut bobject_files_parsed = true;
    let mut bobject_files = Vec::new();
    for (i, maybe_bobject_file) in bobject_file_iter.enumerate() {
        match maybe_bobject_file {
            Ok(bobj) => bobject_files.push(bobj),
            Err(e) => {
                bobject_files_parsed = false;
                eprintln!("Bobject file {} is broken: {e}", args.inputs[i].display());
            }
        }
    }
    if !bobject_files_parsed {
        err!("Error: exiting because of broken bobject file(s)");
    }
    bobject_files
}

fn compute_base_addresses(bobject_files: &[BobjectFile]) -> Vec<u32> {
    let mut curr: u32 = 0;
    let mut base_addresses = Vec::new();
    for bobject_file in bobject_files {
        base_addresses.push(curr);
        curr += bobject_file.word_count() as u32;
    }
    if curr > 0x10000 {
        err!(
            "Error: linked program is {} words, but maximum is 65536",
            curr
        );
    }
    base_addresses
}

fn build_global_symbol_table(
    bobject_files: &[BobjectFile],
    base_addresses: &[u32],
) -> HashMap<String, u16> {
    let mut table = HashMap::new();
    let mut ok = true;
    for (i, bobject_file) in bobject_files.iter().enumerate() {
        for sym in &bobject_file.symbols {
            if sym.binding != Binding::Global {
                continue;
            }
            let addr = (base_addresses[i] + sym.offset as u32) as u16;
            if table.insert(sym.name.clone(), addr).is_some() {
                ok = false;
                eprintln!("Error: duplicate global symbol '{}'", sym.name);
            }
        }
    }
    if !ok {
        err!("Error: exiting because of duplicate global symbol(s)");
    }
    table
}

struct ResolvedRelocation {
    object_index: usize,
    site: u16,
    kind: RelocationKind,
    target: u16,
}

fn resolve_relocations(
    bobject_files: &[BobjectFile],
    base_addresses: &[u32],
    global_symbol_table: &HashMap<String, u16>,
) -> Vec<ResolvedRelocation> {
    let mut resolved = Vec::new();
    let mut ok = true;
    for (i, bobject_file) in bobject_files.iter().enumerate() {
        for Relocation { site, kind, name } in &bobject_file.relocations {
            if let Some(sym) = bobject_file.find_symbol(name) {
                let target = (base_addresses[i] + sym.offset as u32) as u16;
                resolved.push(ResolvedRelocation {
                    object_index: i,
                    site: *site,
                    kind: *kind,
                    target,
                })
            } else if let Some(target) = global_symbol_table.get(name) {
                resolved.push(ResolvedRelocation {
                    object_index: i,
                    site: *site,
                    kind: *kind,
                    target: *target,
                })
            } else {
                ok = false;
                eprintln!("Error: undefined symbol '{}'", name);
            };
        }
    }
    if !ok {
        err!("Error: exiting because of undefined symbol(s)");
    }
    resolved
}

fn compute_fixup_values(
    base_addresses: &[u32],
    resolved_relocations: &[ResolvedRelocation],
) -> Vec<u16> {
    let mut out = Vec::new();
    let mut ok = true;
    for r in resolved_relocations {
        let result = match r.kind {
            RelocationKind::Abs16 => Ok(r.target),
            RelocationKind::PcRel9 => {
                pc_rel_fixup(r.target, base_addresses[r.object_index], r.site, 9)
            }
            RelocationKind::PcRel11 => {
                pc_rel_fixup(r.target, base_addresses[r.object_index], r.site, 11)
            }
        };
        match result {
            Ok(fixup) => out.push(fixup),
            Err(msg) => {
                ok = false;
                eprintln!("Error: {msg}");
            }
        }
    }
    if !ok {
        err!("Error: exiting because of relocation(s) being out of range");
    }
    out
}

fn pc_rel_fixup(target: u16, base: u32, site: u16, bits: u32) -> Result<u16, String> {
    let site_addr = (base as u16).wrapping_add(site).wrapping_add(1);
    let fixup = target.wrapping_sub(site_addr) as i16;

    let (lo, hi) = (-(1 << (bits - 1)), (1 << (bits - 1)) - 1);
    if fixup < lo || fixup > hi {
        return Err(format!("relocation out of range ({fixup} in {bits} bits)"));
    }
    Ok(fixup as u16)
}

fn patch(
    fixup_values: &[u16],
    words: &mut [Vec<u16>],
    resolved_relocations: &[ResolvedRelocation],
) {
    for (
        i,
        ResolvedRelocation {
            object_index,
            kind,
            site,
            ..
        },
    ) in resolved_relocations.iter().enumerate()
    {
        let fixup_value = fixup_values[i];
        let obj_words = &mut words[*object_index];
        match kind {
            RelocationKind::Abs16 => {
                obj_words[*site as usize] = fixup_value;
            }
            RelocationKind::PcRel9 => {
                let existing_word = obj_words[*site as usize];
                obj_words[*site as usize] = (existing_word & 0xFE00) | (fixup_value & 0x01FF);
            }
            RelocationKind::PcRel11 => {
                let existing_word = obj_words[*site as usize];
                obj_words[*site as usize] = (existing_word & 0xF800) | (fixup_value & 0x07FF);
            }
        }
    }
}
