use bobj::bobject::{BobjectFile, Relocation, RelocationKind};

use crate::{
    parser::{LineContent, Operand, ParsedInstr, SourceLine},
    pass1::LayoutResult,
};

pub fn generate_bobject(ast: Vec<SourceLine>, layout: LayoutResult) -> BobjectFile {
    let mut bobject_file = BobjectFile {
        symbols: layout.symbols,
        relocations: Vec::new(),
        words: Vec::new(),
    };

    let mut resolve_operand = |operand: Operand, reloc_kind: RelocationKind, offset: usize| {
        match operand {
            // relocation kind can be used to get the number of bits
            Operand::Imm(i) => match reloc_kind {
                RelocationKind::Abs16 => i as i16 as u16 & 0xFFFF,
                RelocationKind::PcRel9 => i as i16 as u16 & 0x01FF,
                RelocationKind::PcRel11 => i as i16 as u16 & 0x07FF,
            },
            Operand::Label(l) => {
                bobject_file.relocations.push(Relocation {
                    site: offset as u16,
                    kind: reloc_kind,
                    name: l,
                });
                0 // Gets filled in in the linker
            }
        }
    };

    for (line, offset) in ast.into_iter().zip(layout.offsets.into_iter()) {
        if let Some(line_content) = line.content {
            match line_content {
                LineContent::Instr(inst) => match inst {
                    ParsedInstr::Nop => bobject_file.words.push(0x0000),

                    ParsedInstr::AddRR { dst, src1, src2 } => bobject_file.words.push(
                        0x1000
                            | (dst as u16) << 9
                            | (0b00 << 7)
                            | (src1 as u16) << 4
                            | (src2 as u16) << 1,
                    ),
                    ParsedInstr::AddRI { dst, src1, imm } => bobject_file.words.push(
                        0x1000
                            | (dst as u16) << 9
                            | (0b01 << 7)
                            | (src1 as u16) << 4
                            | (imm as i16 as u16 & 0b1111),
                    ),
                    ParsedInstr::AddIpR { dst, src } => bobject_file
                        .words
                        .push(0x1000 | (dst as u16) << 9 | (0b10 << 7) | (src as u16) << 4),
                    ParsedInstr::AddIpI { dst, imm } => bobject_file.words.push(
                        0x1000 | (dst as u16) << 9 | (0b11 << 7) | (imm as i16 as u16 & 0b1111111),
                    ),

                    ParsedInstr::AndRR { dst, src1, src2 } => bobject_file.words.push(
                        0x2000
                            | (dst as u16) << 9
                            | (0b00 << 7)
                            | (src1 as u16) << 4
                            | (src2 as u16) << 1,
                    ),
                    ParsedInstr::AndRI { dst, src1, imm } => bobject_file.words.push(
                        0x2000
                            | (dst as u16) << 9
                            | (0b01 << 7)
                            | (src1 as u16) << 4
                            | (imm as i16 as u16 & 0b1111),
                    ),
                    ParsedInstr::AndIpR { dst, src } => bobject_file
                        .words
                        .push(0x2000 | (dst as u16) << 9 | (0b10 << 7) | (src as u16) << 4),
                    ParsedInstr::AndIpI { dst, imm } => bobject_file.words.push(
                        0x2000 | (dst as u16) << 9 | (0b11 << 7) | (imm as i16 as u16 & 0b1111111),
                    ),

                    ParsedInstr::NotR { dst, src } => bobject_file
                        .words
                        .push(0x3000 | (dst as u16) << 9 | (0b0 << 8) | (src as u16) << 5),
                    ParsedInstr::NotIp { dst } => bobject_file
                        .words
                        .push(0x3000 | (dst as u16) << 9 | (0b1 << 8)),

                    ParsedInstr::Ld { dst, target } => {
                        let imm = resolve_operand(target, RelocationKind::PcRel9, offset);
                        bobject_file.words.push(0x4000 | (dst as u16) << 9 | imm);
                    }
                    ParsedInstr::Ldi { dst, target } => {
                        let imm = resolve_operand(target, RelocationKind::PcRel9, offset);
                        bobject_file.words.push(0x5000 | (dst as u16) << 9 | imm);
                    }
                    ParsedInstr::Ldr { dst, base, imm } => bobject_file.words.push(
                        0x6000
                            | (dst as u16) << 9
                            | (base as u16) << 6
                            | (imm as i16 as u16 & 0x3F),
                    ),
                    ParsedInstr::St { src, target } => {
                        let imm = resolve_operand(target, RelocationKind::PcRel9, offset);
                        bobject_file.words.push(0x7000 | (src as u16) << 9 | imm);
                    }
                    ParsedInstr::Sti { src, target } => {
                        let imm = resolve_operand(target, RelocationKind::PcRel9, offset);
                        bobject_file.words.push(0x8000 | (src as u16) << 9 | imm);
                    }
                    ParsedInstr::Str { src, base, imm } => bobject_file.words.push(
                        0x9000
                            | (src as u16) << 9
                            | (base as u16) << 6
                            | (imm as i16 as u16 & 0x3F),
                    ),
                    ParsedInstr::Br { n, z, p, target } => {
                        let imm = resolve_operand(target, RelocationKind::PcRel9, offset);
                        bobject_file.words.push(
                            0xA000 | (n as u16) << 11 | (z as u16) << 10 | (p as u16) << 9 | imm,
                        );
                    }
                    ParsedInstr::Jmp { src } => bobject_file.words.push(0xB000 | (src as u16) << 9),
                    ParsedInstr::Jsr { target } => {
                        let imm = resolve_operand(target, RelocationKind::PcRel11, offset);
                        bobject_file.words.push(0xC000 | imm);
                    }
                    ParsedInstr::JsrR { src } => bobject_file
                        .words
                        .push(0xC000 | (1 << 11) | (src as u16) << 8),
                    ParsedInstr::Lea { dst, target } => {
                        let imm = resolve_operand(target, RelocationKind::PcRel9, offset);
                        bobject_file.words.push(0xD000 | (dst as u16) << 9 | imm);
                    }
                    ParsedInstr::Ret => bobject_file.words.push(0xE000),
                    ParsedInstr::Trap { vector } => {
                        bobject_file.words.push(0xF000 | (vector as u16) << 8)
                    }
                },
                LineContent::FillDirective(operand) => {
                    bobject_file
                        .words
                        .push(resolve_operand(operand, RelocationKind::Abs16, offset))
                }
                LineContent::StringDirective(string) => {
                    for c in string.chars() {
                        bobject_file.words.push(c as u8 as u16);
                    }
                    bobject_file.words.push(0); // null terminator -- word_count() already counted this
                }
                LineContent::GlobalDirective(_) => {}
            }
        }
    }

    bobject_file
}
