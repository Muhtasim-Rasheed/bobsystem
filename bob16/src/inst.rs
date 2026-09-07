pub enum Instruction {
    Nop,
    AddRR { dst: u8, src1: u8, src2: u8 },
    AddRI { dst: u8, src1: u8, imm: i8 },
    AddIpR { dst: u8, src: u8 },
    AddIpI { dst: u8, imm: i8 },
    AndRR { dst: u8, src1: u8, src2: u8 },
    AndRI { dst: u8, src1: u8, imm: i8 },
    AndIpR { dst: u8, src: u8 },
    AndIpI { dst: u8, imm: i8 },
    NotR { dst: u8, src: u8 },
    NotIp { dst: u8 },
    Ld { dst: u8, imm: i16 },
    Ldi { dst: u8, imm: i16 },
    Ldr { dst: u8, base: u8, imm: i16 },
    St { src: u8, imm: i16 },
    Sti { src: u8, imm: i16 },
    Str { src: u8, base: u8, imm: i16 },
    Br { n: bool, z: bool, p: bool, imm: i16 },
    Jmp { src: u8 },
    Jsr { imm: i16 },
    JsrR { src: u8 },
    Lea { dst: u8, imm: i16 },
    Ret,
    Trap { vector: u8 },
}

impl From<u16> for Instruction {
    fn from(value: u16) -> Self {
        let opcode = bits(value, 15, 12);
        match opcode {
            0b0000 => Self::Nop,
            0b0001 => {
                let mode = bits(value, 8, 7);
                let dst = bits(value, 11, 9) as u8;
                match mode {
                    0b00 => Self::AddRR {
                        dst,
                        src1: bits(value, 6, 4) as u8,
                        src2: bits(value, 3, 1) as u8,
                    },
                    0b01 => Self::AddRI {
                        dst,
                        src1: bits(value, 6, 4) as u8,
                        imm: sext(bits(value, 3, 0), 4) as i8,
                    },
                    0b10 => Self::AddIpR {
                        dst,
                        src: bits(value, 6, 4) as u8,
                    },
                    0b11 => Self::AddIpI {
                        dst,
                        imm: sext(bits(value, 6, 0), 7) as i8,
                    },
                    _ => unreachable!(),
                }
            }
            0b0010 => {
                let mode = bits(value, 8, 7);
                let dst = bits(value, 11, 9) as u8;
                match mode {
                    0b00 => Self::AndRR {
                        dst,
                        src1: bits(value, 6, 4) as u8,
                        src2: bits(value, 3, 1) as u8,
                    },
                    0b01 => Self::AndRI {
                        dst,
                        src1: bits(value, 6, 4) as u8,
                        imm: sext(bits(value, 3, 0), 4) as i8,
                    },
                    0b10 => Self::AndIpR {
                        dst,
                        src: bits(value, 6, 4) as u8,
                    },
                    0b11 => Self::AndIpI {
                        dst,
                        imm: sext(bits(value, 6, 0), 7) as i8,
                    },
                    _ => unreachable!(),
                }
            }
            0b0011 => {
                let mode = bits(value, 8, 8);
                let dst = bits(value, 11, 9) as u8;
                match mode {
                    0b0 => Self::NotR {
                        dst,
                        src: bits(value, 7, 5) as u8,
                    },
                    0b1 => Self::NotIp { dst },
                    _ => unreachable!(),
                }
            }
            0b0100 => {
                let dst = bits(value, 11, 9) as u8;
                let imm = sext(bits(value, 8, 0), 9);
                Self::Ld { dst, imm }
            }
            0b0101 => {
                let dst = bits(value, 11, 9) as u8;
                let imm = sext(bits(value, 8, 0), 9);
                Self::Ldi { dst, imm }
            }
            0b0110 => {
                let dst = bits(value, 11, 9) as u8;
                let base = bits(value, 8, 6) as u8;
                let imm = sext(bits(value, 5, 0), 6);
                Self::Ldr { dst, base, imm }
            }
            0b0111 => {
                let src = bits(value, 11, 9) as u8;
                let imm = sext(bits(value, 8, 0), 9);
                Self::St { src, imm }
            }
            0b1000 => {
                let src = bits(value, 11, 9) as u8;
                let imm = sext(bits(value, 8, 0), 9);
                Self::Sti { src, imm }
            }
            0b1001 => {
                let src = bits(value, 11, 9) as u8;
                let base = bits(value, 8, 6) as u8;
                let imm = sext(bits(value, 5, 0), 6);
                Self::Str { src, base, imm }
            }
            0b1010 => {
                let n = bits(value, 11, 11) == 1;
                let z = bits(value, 10, 10) == 1;
                let p = bits(value, 9, 9) == 1;
                let imm = sext(bits(value, 8, 0), 9);
                Self::Br { n, z, p, imm }
            }
            0b1011 => {
                let src = bits(value, 11, 9) as u8;
                Self::Jmp { src }
            }
            0b1100 => {
                let mode = bits(value, 11, 11);
                match mode {
                    0b0 => Self::Jsr {
                        imm: sext(bits(value, 10, 0), 11),
                    },
                    0b1 => Self::JsrR {
                        src: bits(value, 10, 8) as u8,
                    },
                    _ => unreachable!(),
                }
            }
            0b1101 => {
                let dst = bits(value, 11, 9) as u8;
                let imm = sext(bits(value, 8, 0), 9) as i16;
                Self::Lea { dst, imm }
            }
            0b1110 => Self::Ret,
            0b1111 => {
                let vector = bits(value, 11, 8) as u8;
                Self::Trap { vector }
            }
            _ => unreachable!(),
        }
    }
}

fn bits(val: u16, hi: u16, lo: u16) -> u16 {
    (val >> lo) & ((1 << (hi - lo + 1)) - 1)
}

fn sext(val: u16, width: u16) -> i16 {
    let shift = 16 - width;
    ((val << shift) as i16) >> shift
}
