use std::{collections::HashMap, fmt::Write as _};

use crate::{
    op::{BinaryOp, UnaryOp},
    rast::{FuncId, VarId},
    tac::{Label, TacAsmOperand, TacFunction, TacInst, TacProgram, TacRegister, Temp},
};

fn runtime_symbol(op: BinaryOp) -> Option<&'static str> {
    match op {
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::BitAnd => None, // inlined
        BinaryOp::Mul => Some("__rt_mul"),
        BinaryOp::Div => Some("__rt_div"),
        BinaryOp::Mod => Some("__rt_mod"),
        BinaryOp::BitOr => Some("__rt_bitor"),
        BinaryOp::BitXor => Some("__rt_bitxor"),
        BinaryOp::LogAnd => Some("__rt_logand"),
        BinaryOp::LogOr => Some("__rt_logor"),
        BinaryOp::Eq => Some("__rt_eq"),
        BinaryOp::Ne => Some("__rt_ne"),
        BinaryOp::Lt => Some("__rt_lt"),
        BinaryOp::Le => Some("__rt_le"),
        BinaryOp::Gt => Some("__rt_gt"),
        BinaryOp::Ge => Some("__rt_ge"),
    }
}

fn reg_name(r: TacRegister) -> &'static str {
    match r {
        TacRegister::R0 => "r0",
        TacRegister::R1 => "r1",
        TacRegister::R2 => "r2",
        TacRegister::R3 => "r3",
        TacRegister::R4 => "r4",
        TacRegister::R5 => "r5",
        TacRegister::R6 => "r6",
        TacRegister::R7 => "r7",
    }
}

#[derive(Default)]
struct LiteralPool {
    next_id: u32,
    strings: Vec<(String, String)>, // (label, content)
    ints: Vec<(String, u16)>,       // (label, value)
    funcs: Vec<(String, String)>,   // (label, name)
}
impl LiteralPool {
    fn intern_string(&mut self, s: &str) -> String {
        for (label, content) in &self.strings {
            if content == s {
                return label.clone();
            }
        }
        let label = format!("__lstr{}", self.next_id);
        self.next_id += 1;
        self.strings.push((label.clone(), s.to_string()));
        label
    }
    fn intern_int(&mut self, i: u16) -> String {
        for (label, content) in &self.ints {
            if *content == i {
                return label.clone();
            }
        }
        let label = format!("__lint{}", self.next_id);
        self.next_id += 1;
        self.ints.push((label.clone(), i));
        label
    }
    fn intern_func(&mut self, f: String) -> String {
        for (label, name) in &self.funcs {
            if *name == f {
                return label.clone();
            }
        }
        let label = format!("__lfn{}", self.next_id);
        self.next_id += 1;
        self.funcs.push((label.clone(), f));
        label
    }
    fn emit(&self, out: &mut String) {
        for (label, content) in &self.strings {
            let escaped = content
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n");
            let _ = writeln!(out, "{label}: .string \"{escaped}\"");
        }
        for (label, val) in &self.ints {
            let _ = writeln!(out, "{label}: .fill {val}");
        }
        for (label, name) in &self.funcs {
            let _ = writeln!(out, "{label}: .fill {name}");
        }
    }
    fn size(&self) -> usize {
        self.strings
            .iter()
            .map(|(_, s)| s.chars().count() + 1)
            .sum::<usize>()
            + self.ints.len()
            + self.funcs.len()
    }
    fn clear(&mut self) {
        self.strings.clear();
        self.ints.clear();
        self.funcs.clear();
    }
}

const MAX_CHUNK_WORDS: usize = 200;

#[derive(Default)]
struct Chunk {
    words: usize,
    pool: LiteralPool,
}

impl Chunk {
    fn clear(&mut self) {
        self.words = 0;
        self.pool.clear();
    }
}

#[derive(Default)]
struct FrameLayout {
    var_offsets: HashMap<VarId, u16>,
}

impl FrameLayout {
    fn clear(&mut self) {
        self.var_offsets.clear();
    }
}

pub struct Codegen<'a> {
    func_label: &'a dyn Fn(FuncId) -> String,
    current_chunk: Chunk,
    chunk_count: usize,
    frame_layout: FrameLayout,
    out: String,
}

impl<'a> Codegen<'a> {
    pub fn new(func_label: &'a dyn Fn(FuncId) -> String) -> Self {
        Self {
            func_label,
            current_chunk: Chunk::default(),
            chunk_count: 0,
            frame_layout: FrameLayout::default(),
            out: String::new(),
        }
    }

    pub fn generate(mut self, program: &TacProgram) -> String {
        for func in &program.funcs {
            self.gen_function(func);
        }
        self.out
    }

    fn label_for(&self, id: FuncId) -> String {
        (self.func_label)(id)
    }

    fn frame_size(func: &TacFunction) -> u32 {
        func.locals.len() as u32 + func.num_temps
    }

    fn temp_offset(func: &TacFunction, t: Temp) -> u16 {
        (func.locals.len() as u32 + t.0) as u16
    }
    fn var_offset(&mut self, v: crate::rast::VarId) -> u16 {
        let next_offset = self.frame_layout.var_offsets.len() as u16;
        *self
            .frame_layout
            .var_offsets
            .entry(v)
            .or_insert(next_offset)
    }

    fn gen_function(&mut self, func: &TacFunction) {
        let label = self.label_for(func.id);
        let extra = func.locals.len() as u32 - func.params.len() as u32 + func.num_temps;

        self.frame_layout.clear();

        for (offset, var_id) in func.params.iter().enumerate() {
            self.frame_layout.var_offsets.insert(*var_id, offset as u16);
        }

        let _ = writeln!(self.out, ".global {label}");
        let _ = writeln!(self.out, "{label}:");
        self.emit_inst("str r5, r4, 0");
        self.emit_inst("str r7, r4, 1");
        self.emit_inst("add r4, 2");
        self.emit_inst(format!("add r5, r6, -{}", func.params.len()));
        if extra > 0 {
            self.emit_inst(format!("add r6, {extra}"));
        }

        for inst in &func.body {
            self.gen_inst(func, inst);
        }

        self.gen_epilogue(func, None);
        self.finish_chunk();
    }

    fn gen_epilogue(&mut self, func: &TacFunction, ret: Option<Temp>) {
        if let Some(t) = ret {
            let off = Self::temp_offset(func, t);
            self.emit_inst(format!("ldr r0, r5, {off}"));
        }
        let size = Self::frame_size(func);
        if size > 0 {
            self.emit_inst(format!("add r6, -{size}"));
        }
        self.emit_inst("add r4, -2");
        self.emit_inst("ldr r7, r4, 1");
        self.emit_inst("ldr r5, r4, 0");
        self.emit_inst("ret");
    }

    fn load_temp(&mut self, func: &TacFunction, dst_reg: &str, t: Temp) {
        let off = Self::temp_offset(func, t);
        self.emit_inst(format!("ldr {dst_reg}, r5, {off}"));
    }
    fn store_temp(&mut self, func: &TacFunction, src_reg: &str, t: Temp) {
        let off = Self::temp_offset(func, t);
        self.emit_inst(format!("str {src_reg}, r5, {off}"));
    }

    fn gen_inst(&mut self, func: &TacFunction, inst: &TacInst) {
        self.maybe_finish_chunk();
        match inst {
            TacInst::ConstInt { dst, val } => {
                let label = self.current_chunk.pool.intern_int(*val);
                self.emit_inst(format!("ld r0, {label}"));
                self.store_temp(func, "r0", *dst);
            }
            TacInst::ConstStr { dst, val } => {
                let label = self.current_chunk.pool.intern_string(val);
                self.emit_inst(format!("lea r0, {label}"));
                self.store_temp(func, "r0", *dst);
            }
            TacInst::Copy { dst, src } => {
                self.load_temp(func, "r0", *src);
                self.store_temp(func, "r0", *dst);
            }
            TacInst::LoadVar { dst, var } => {
                let off = self.var_offset(*var);
                self.emit_inst(format!("ldr r0, r5, {off}"));
                self.store_temp(func, "r0", *dst);
            }
            TacInst::StoreVar { var, src } => {
                self.load_temp(func, "r0", *src);
                let off = self.var_offset(*var);
                self.emit_inst(format!("str r0, r5, {off}"));
            }
            TacInst::Load { dst, addr } => {
                self.load_temp(func, "r0", *addr);
                self.emit_inst("ldr r0, r0, 0");
                self.store_temp(func, "r0", *dst);
            }
            TacInst::Store { addr, src } => {
                self.load_temp(func, "r0", *addr);
                self.load_temp(func, "r1", *src);
                self.emit_inst(format!("str r1, r0, 0"));
            }
            TacInst::AddrOf { dst, var } => {
                let off = self.var_offset(*var);
                self.emit_inst(format!("add r0, r5, {off}"));
                self.store_temp(func, "r0", *dst);
            }
            TacInst::LoadFunc { dst, func: target } => {
                let ptr_label = self
                    .current_chunk
                    .pool
                    .intern_func((self.func_label)(*target));
                self.emit_inst(format!("ld r0, {ptr_label}"));
                self.store_temp(func, "r0", *dst);
            }
            TacInst::BinOp { dst, op, lhs, rhs } => self.gen_binop(func, *dst, *op, *lhs, *rhs),
            TacInst::UnOp { dst, op, src } => self.gen_unop(func, *dst, *op, *src),
            TacInst::Param(t) => {
                self.load_temp(func, "r0", *t);
                self.emit_inst("str r0, r6, 0");
                self.emit_inst("add r6, 1");
            }
            TacInst::ParamInt(i) => {
                let label = self.current_chunk.pool.intern_int(*i);
                self.emit_inst(format!("ld r0, {label}"));
                self.emit_inst("str r0, r6, 0");
                self.emit_inst("add r6, 1");
            }
            TacInst::ParamStr(s) => {
                let label = self.current_chunk.pool.intern_string(&s);
                self.emit_inst(format!("lea r0, {label}"));
                self.emit_inst("str r0, r6, 0");
                self.emit_inst("add r6, 1");
            }
            TacInst::Call { dst, callee } => {
                self.load_temp(func, "r0", *callee);
                self.emit_inst("jsr r0");
                if let Some(dst) = dst {
                    self.store_temp(func, "r0", *dst);
                }
            }
            TacInst::CallDirect { dst, callee } => {
                let ptr_label = self
                    .current_chunk
                    .pool
                    .intern_func((self.func_label)(*callee));
                self.emit_inst(format!("ld r0, {ptr_label}"));
                self.emit_inst(format!("jsr r0"));
                if let Some(dst) = dst {
                    self.store_temp(func, "r0", *dst);
                }
            }
            TacInst::Label(Label(id)) => {
                let _ = writeln!(self.out, "__{}L{id}:", func.id.0);
            }
            TacInst::Jump(Label(id)) => {
                self.emit_inst(format!("br __{}L{id}", func.id.0));
            }
            TacInst::JumpIfZero {
                cond,
                target: Label(id),
            } => {
                self.load_temp(func, "r0", *cond);
                self.emit_inst(format!("brz __{}L{id}", func.id.0));
            }
            TacInst::Return(val) => self.gen_epilogue(func, *val),
            TacInst::AsmIn { dst, src } => {
                self.load_temp(func, reg_name(*dst), *src);
            }
            TacInst::AsmOut { dst, src } => {
                self.store_temp(func, reg_name(*src), *dst);
            }
            TacInst::AsmRaw { mnemonic, args } => {
                let operands: Vec<String> = args
                    .iter()
                    .map(|a| match a {
                        TacAsmOperand::Reg(r) => reg_name(*r).to_string(),
                        TacAsmOperand::Imm(n) => n.to_string(),
                    })
                    .collect();
                self.emit_inst(format!("{mnemonic} {}", operands.join(" ")));
            }
            TacInst::Dummy => {}
        }
    }

    fn emit_inst(&mut self, line: impl std::fmt::Display) {
        let _ = writeln!(self.out, "\t{line}");
        self.current_chunk.words += 1;
    }

    fn maybe_finish_chunk(&mut self) {
        if self.current_chunk.words + self.current_chunk.pool.size() >= MAX_CHUNK_WORDS {
            self.finish_chunk();
        }
    }

    fn finish_chunk(&mut self) {
        let pool_id = self.chunk_count;
        let after_pool = format!("__after_pool{pool_id}");
        self.emit_inst(format!("br {after_pool}"));
        self.current_chunk.pool.emit(&mut self.out);
        let _ = writeln!(self.out, "{after_pool}:");
        self.current_chunk.clear();
        self.chunk_count += 1;
    }

    fn gen_binop(&mut self, func: &TacFunction, dst: Temp, op: BinaryOp, lhs: Temp, rhs: Temp) {
        if let Some(sym) = runtime_symbol(op) {
            let ptr_label = self.current_chunk.pool.intern_func(sym.to_string()); // the runtime should implement the operations
            self.load_temp(func, "r0", lhs);
            self.emit_inst("str r0, r6, 0");
            self.emit_inst("add r6, 1");
            self.load_temp(func, "r0", rhs);
            self.emit_inst("str r0, r6, 0");
            self.emit_inst("add r6, 1");
            self.emit_inst(format!("ld r0, {ptr_label}"));
            self.emit_inst("jsr r0");
            self.store_temp(func, "r0", dst);
            return;
        }
        self.load_temp(func, "r0", lhs);
        self.load_temp(func, "r1", rhs);
        match op {
            BinaryOp::Add => {
                self.emit_inst("add r0, r1");
            }
            BinaryOp::BitAnd => {
                self.emit_inst("and r0, r1");
            }
            BinaryOp::Sub => {
                self.emit_inst("not r1");
                self.emit_inst("add r1, 1");
                self.emit_inst("add r0, r1");
            }
            _ => unreachable!("non-inline op should have gone through runtime_symbol"),
        }
        self.store_temp(func, "r0", dst);
    }

    fn gen_unop(&mut self, func: &TacFunction, dst: Temp, op: UnaryOp, src: Temp) {
        self.load_temp(func, "r0", src);
        match op {
            UnaryOp::BitNot => {
                self.emit_inst("not r0");
            }
            UnaryOp::Neg => {
                self.emit_inst("not r0");
                self.emit_inst("add r0, 1");
            }
            UnaryOp::LogNot => {
                self.emit_inst("str r0, r6, 0");
                self.emit_inst("add r6, 1");
                let label = self
                    .current_chunk
                    .pool
                    .intern_func("__rt_lognot".to_string());
                self.emit_inst(format!("ld r0, {label}"));
                self.emit_inst("jsr r0");
            }
        }
        self.store_temp(func, "r0", dst);
    }
}
