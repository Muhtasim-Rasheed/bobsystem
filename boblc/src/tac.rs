use crate::{
    op::{BinaryOp, UnaryOp},
    rast::{FuncId, VarId},
    ty::Ty,
};

pub struct TacProgram {
    pub funcs: Vec<TacFunction>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Temp(pub u32);
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Label(pub u32);

#[derive(Clone, PartialEq, Eq)]
pub enum TacInst {
    ConstInt {
        dst: Temp,
        val: u16,
    },
    ConstStr {
        dst: Temp,
        val: String,
    },
    #[allow(unused)]
    Copy {
        dst: Temp,
        src: Temp,
    },
    LoadVar {
        dst: Temp,
        var: VarId,
    },
    StoreVar {
        var: VarId,
        src: Temp,
    },
    Load {
        dst: Temp,
        addr: Temp,
    },
    Store {
        addr: Temp,
        src: Temp,
    },
    AddrOf {
        dst: Temp,
        var: VarId,
    },
    LoadFunc {
        dst: Temp,
        func: FuncId,
    },
    BinOp {
        dst: Temp,
        op: BinaryOp,
        lhs: Temp,
        rhs: Temp,
    },
    UnOp {
        dst: Temp,
        op: UnaryOp,
        src: Temp,
    },
    Param(Temp),
    ParamInt(u16),
    ParamStr(String),
    Call {
        dst: Option<Temp>,
        callee: Temp,
    },
    CallDirect {
        dst: Option<Temp>,
        callee: FuncId,
    },
    Label(Label),
    Jump(Label),
    JumpIfZero {
        cond: Temp,
        target: Label,
    },
    Return(Option<Temp>),
    AsmIn {
        dst: TacRegister,
        src: Temp,
    },
    AsmOut {
        dst: Temp,
        src: TacRegister,
    },
    AsmRaw {
        mnemonic: String,
        args: Vec<TacAsmOperand>,
    },
    Dummy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TacRegister {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
}

#[derive(Clone, PartialEq, Eq)]
pub enum TacAsmOperand {
    Reg(TacRegister),
    Imm(u16),
}

pub struct TacFunction {
    pub id: FuncId,
    pub params: Vec<VarId>,
    pub num_temps: u32,
    pub locals: Vec<Ty>,
    pub body: Vec<TacInst>,
}

impl std::fmt::Debug for Temp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "t{}", self.0)
    }
}
impl std::fmt::Debug for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "l{}", self.0)
    }
}

impl std::fmt::Debug for TacAsmOperand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reg(reg) => write!(f, "{reg:?}"),
            Self::Imm(imm) => write!(f, "{imm:?}"),
        }
    }
}

impl std::fmt::Debug for TacInst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConstInt { dst, val } => write!(f, "    {dst:?} = {val:?}"),
            Self::ConstStr { dst, val } => write!(f, "    {dst:?} = {val:?}"),
            Self::Copy { dst, src } => write!(f, "    {dst:?} = {src:?}"),
            Self::LoadVar { dst, var } => write!(f, "    {dst:?} = v{:?}", var.0),
            Self::StoreVar { var, src } => write!(f, "    v{:?} = {src:?}", var.0),
            Self::Load { dst, addr } => write!(f, "    {dst:?} = *{addr:?}"),
            Self::Store { addr, src } => write!(f, "    *{addr:?} = {src:?}"),
            Self::AddrOf { dst, var } => write!(f, "    {dst:?} = &{var:?}"),
            Self::LoadFunc { dst, func } => write!(f, "    {dst:?} = f{:?}", func.0),
            Self::BinOp { dst, op, lhs, rhs } => write!(f, "    {dst:?} = {lhs:?} {op} {rhs:?}"),
            Self::UnOp { dst, op, src } => write!(f, "    {dst:?} = {op}{src:?}"),
            Self::Param(param) => write!(f, "    param {param:?}"),
            Self::ParamInt(param) => write!(f, "    param {param:?}"),
            Self::ParamStr(param) => write!(f, "    param {param:?}"),
            Self::Call { dst, callee } => write!(f, "    {dst:?} = call {callee:?}"),
            Self::CallDirect { dst, callee } => write!(f, "    {dst:?} = call f{}", callee.0),
            Self::Label(label) => write!(f, "{label:?}:"),
            Self::Jump(label) => write!(f, "    jump {label:?}"),
            Self::JumpIfZero { cond, target } => write!(f, "    jump {target:?} if {cond:?} == 0"),
            Self::Return(ret) => write!(f, "    return {:?}", ret.map(|v| format!("{v:?}"))),
            Self::AsmIn { dst, src } => write!(f, "    {dst:?} = {src:?}"),
            Self::AsmOut { dst, src } => write!(f, "    {dst:?} = {src:?}"),
            Self::AsmRaw { mnemonic, args } => write!(f, "    {mnemonic} {args:?}"),
            Self::Dummy => write!(f, "    dummy"),
        }
    }
}

impl std::fmt::Debug for TacFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "f{} {{", self.id.0)?;
        for inst in &self.body {
            writeln!(f, "    {inst:?}")?;
        }
        writeln!(f, "}}")
    }
}

impl std::fmt::Debug for TacProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for func in &self.funcs {
            writeln!(f, "{:?}", func)?;
        }
        Ok(())
    }
}
