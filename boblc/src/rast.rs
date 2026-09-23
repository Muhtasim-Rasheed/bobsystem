#![allow(unused)]

use crate::{
    op::{BinaryOp, UnaryOp},
    ty::Ty,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub u32);

#[derive(Debug)]
pub struct RProgram {
    pub modules: Vec<RModule>,
}

#[derive(Debug)]
pub struct RModule {
    pub id: ModuleId,
    pub functions: Vec<RFunction>,
}

#[derive(Debug)]
pub struct RFunction {
    pub id: FuncId,
    pub module: ModuleId,
    pub name: String,
    pub params: Vec<(VarId, Ty)>,
    pub ret: Ty,
    pub locals: Vec<Ty>,
    pub body: RStmt,
}

#[derive(Debug)]
pub enum RStmt {
    Let {
        var: VarId,
        val: Option<RExpr>,
    },
    Block(Vec<RStmt>),
    If {
        cond: RExpr,
        then_branch: Box<RStmt>,
        else_branch: Option<Box<RStmt>>,
    },
    While {
        cond: RExpr,
        body: Box<RStmt>,
    },
    Expr(RExpr),
    Asm(Vec<RAsmItem>),
    Return(Option<RExpr>),
    Error,
}

#[derive(Debug)]
pub struct RExpr {
    pub kind: RExprKind,
    pub ty: Ty,
}

#[derive(Debug)]
pub enum RExprKind {
    IntLit(u16),
    CharLit(char),
    StrLit(String),
    LValue(RLValue),
    Function(FuncId),
    Ref(RLValue),
    Binary {
        left: Box<RExpr>,
        op: BinaryOp,
        right: Box<RExpr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<RExpr>,
    },
    Assign {
        target: RLValue,
        val: Box<RExpr>,
    },
    Call {
        callee: Box<RExpr>,
        args: Vec<RExpr>,
    },
    Error,
}

#[derive(Debug)]
pub enum RLValue {
    Var(VarId),
    Deref(Box<RExpr>),
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RRegister {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
}

#[derive(Debug)]
pub enum RAsmItem {
    In {
        reg: RRegister,
        expr: RExpr,
    },
    Out {
        reg: RRegister,
        lval: RLValue,
    },
    Instr {
        mnemonic: String,
        args: Vec<RAsmOperand>,
    },
    Error,
}

#[derive(Debug)]
pub enum RAsmOperand {
    Reg(RRegister),
    Imm(u16),
    Error,
}
