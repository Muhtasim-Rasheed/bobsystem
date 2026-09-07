use crate::inst::Instruction;

pub struct Machine {
    regs: Registers,
    pc: u16,
    n: bool,
    z: bool,
    p: bool,
    halted: bool,
    memory: Box<[u16]>,
}

impl Machine {
    pub fn new(memory: Box<[u16]>) -> Self {
        Self {
            regs: Registers { regs: [0; 8] },
            pc: 0,
            n: false,
            z: false,
            p: false,
            halted: true,
            memory,
        }
    }

    pub fn update_flags(&mut self, val: u16) {
        let val = val as i16;
        self.n = val < 0;
        self.z = val == 0;
        self.p = val > 0;
    }

    pub fn fetch(&mut self) -> u16 {
        let fetched = self.memory[self.pc as usize];
        self.pc = self.pc.wrapping_add(1);
        fetched
    }

    pub fn execute(&mut self, inst: Instruction) {
        use Instruction::*;
        match inst {
            Nop => {}
            AddRR { dst, src1, src2 } => {
                self.regs[dst] = self.regs[src1].wrapping_add(self.regs[src2]);
                self.update_flags(self.regs[dst]);
            }
            AddRI { dst, src1, imm } => {
                self.regs[dst] = self.regs[src1].wrapping_add(imm as i16 as u16);
                self.update_flags(self.regs[dst]);
            }
            AddIpR { dst, src } => {
                self.regs[dst] = self.regs[dst].wrapping_add(self.regs[src]);
                self.update_flags(self.regs[dst]);
            }
            AddIpI { dst, imm } => {
                self.regs[dst] = self.regs[dst].wrapping_add(imm as i16 as u16);
                self.update_flags(self.regs[dst]);
            }
            AndRR { dst, src1, src2 } => {
                self.regs[dst] = self.regs[src1] & self.regs[src2];
                self.update_flags(self.regs[dst]);
            }
            AndRI { dst, src1, imm } => {
                self.regs[dst] = (self.regs[src1] as i16 & imm as i16) as u16;
                self.update_flags(self.regs[dst]);
            }
            AndIpR { dst, src } => {
                self.regs[dst] &= self.regs[src];
                self.update_flags(self.regs[dst]);
            }
            AndIpI { dst, imm } => {
                self.regs[dst] = (self.regs[dst] as i16 & imm as i16) as u16;
                self.update_flags(self.regs[dst]);
            }
            NotR { dst, src } => {
                self.regs[dst] = !self.regs[src];
                self.update_flags(self.regs[dst]);
            }
            NotIp { dst } => {
                self.regs[dst] = !self.regs[dst];
                self.update_flags(self.regs[dst]);
            }
            Ld { dst, imm } => {
                let addr = self.pc.wrapping_add(imm as u16);
                self.regs[dst] = self.memory[addr as usize];
                self.update_flags(self.regs[dst]);
            }
            Ldi { dst, imm } => {
                let addr = self.pc.wrapping_add(imm as u16);
                self.regs[dst] = self.memory[self.memory[addr as usize] as usize];
                self.update_flags(self.regs[dst]);
            }
            Ldr { dst, base, imm } => {
                let addr = self.regs[base].wrapping_add(imm as u16);
                self.regs[dst] = self.memory[addr as usize];
                self.update_flags(self.regs[dst]);
            }
            St { src, imm } => {
                let addr = self.pc.wrapping_add(imm as u16);
                self.memory[addr as usize] = self.regs[src];
            }
            Sti { src, imm } => {
                let addr = self.pc.wrapping_add(imm as u16);
                self.memory[self.memory[addr as usize] as usize] = self.regs[src];
            }
            Str { src, base, imm } => {
                let addr = self.regs[base].wrapping_add(imm as u16);
                self.memory[addr as usize] = self.regs[src];
            }
            Br { n, z, p, imm } => {
                if n && self.n || z && self.z || p && self.p {
                    self.pc = self.pc.wrapping_add(imm as u16);
                }
            }
            Jmp { src } => {
                self.pc = self.regs[src];
            }
            Jsr { imm } => {
                self.regs[7] = self.pc;
                self.pc = self.pc.wrapping_add(imm as u16);
            }
            JsrR { src } => {
                self.regs[7] = self.pc;
                self.pc = self.regs[src];
            }
            Lea { dst, imm } => {
                self.regs[dst] = self.pc.wrapping_add(imm as u16);
                self.update_flags(self.regs[dst]);
            }
            Ret => {
                self.pc = self.regs[7];
            }
            Trap { vector } => match vector {
                0b0000 => {
                    self.halted = true;
                }
                0b0001 => {
                    print!("{}", self.regs[0] as u8 as char);
                }
                0b0010 => {
                    let mut addr = self.regs[0];
                    loop {
                        let word = self.memory[addr as usize];
                        if word == 0 {
                            break;
                        }
                        print!("{}", word as u8 as char);
                        addr = addr.wrapping_add(1);
                    }
                    use std::io::Write;
                    std::io::stdout().flush().ok();
                }
                0b0011 => {
                    use std::io::BufRead;
                    let addr = self.regs[0];
                    let max_len = self.regs[1] as usize;
                    let mut line = String::new();
                    std::io::stdin().lock().read_line(&mut line).ok();
                    let n = line.len().min(max_len.saturating_sub(1));
                    for (i, ch) in line.chars().take(n).enumerate() {
                        self.memory[addr.wrapping_add(i as u16) as usize] = ch as u16;
                    }
                    self.memory[addr.wrapping_add(n as u16) as usize] = 0;
                }
                _ => unimplemented!(),
            },
        }
    }

    pub fn cycle(&mut self) {
        if self.halted {
            return;
        }
        let fetched = self.fetch();
        let decoded = Instruction::from(fetched);
        self.execute(decoded);
    }

    pub fn run(&mut self) {
        self.regs = Registers { regs: [0x0000; 8] };
        self.pc = 0x0000;
        (self.n, self.z, self.p) = (false, false, false);
        self.halted = false;
        while !self.halted {
            self.cycle();
        }
    }
}

struct Registers {
    regs: [u16; 8],
}

impl std::fmt::Debug for Registers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, &reg) in self.regs.iter().enumerate() {
            writeln!(f, "r{i}: {reg:#x}")?;
        }
        Ok(())
    }
}

impl std::ops::Index<u8> for Registers {
    type Output = u16;
    fn index(&self, index: u8) -> &Self::Output {
        &self.regs[index as usize]
    }
}

impl std::ops::IndexMut<u8> for Registers {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        &mut self.regs[index as usize]
    }
}
