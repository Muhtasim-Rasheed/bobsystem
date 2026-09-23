use crate::{
    op::{BinaryOp, UnaryOp},
    source::Spanned,
    ty::Ty,
};

pub type Item = Spanned<ItemKind>;
pub type Stmt = Spanned<StmtKind>;
pub type Expr = Spanned<ExprKind>;

#[derive(Debug)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug)]
pub enum ItemKind {
    Import(Spanned<String>),
    FunctionExternal {
        name: Spanned<String>,
        params: Vec<Ty>,
        ret: Ty,
    },
    FunctionDefined {
        name: Spanned<String>,
        params: Vec<(Spanned<String>, Ty)>,
        ret: Ty,
        body: Stmt,
    },
    Error,
}

#[derive(Debug)]
pub enum StmtKind {
    Let {
        name: Spanned<String>,
        ty: Option<Ty>,
        val: Option<Expr>,
    },
    Block(Vec<Stmt>),
    If {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
    },
    Expr(Expr),
    Asm(Vec<Spanned<AsmItem>>),
    Return(Option<Expr>),
    Error,
}

#[derive(Debug)]
pub enum ExprKind {
    IntLit(u16),
    CharLit(char),
    StrLit(String),
    LValue(Box<LValue>),
    Ref(Spanned<Box<LValue>>),
    Path(Vec<Spanned<String>>),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Assign {
        lval: Spanned<Box<LValue>>,
        expr: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        params: Vec<Expr>,
    },
    Error,
}

#[derive(Debug)]
pub enum LValue {
    Var(String),
    Deref(Expr),
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
}

impl Register {
    pub fn from_name(name: &str) -> Option<Register> {
        match name {
            "r0" => Some(Register::R0),
            "r1" => Some(Register::R1),
            "r2" => Some(Register::R2),
            "r3" => Some(Register::R3),
            "r4" => Some(Register::R4),
            "r5" => Some(Register::R5),
            "r6" => Some(Register::R6),
            "r7" => Some(Register::R7),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum AsmItem {
    In {
        reg: Spanned<Register>,
        expr: Expr,
    },
    Out {
        reg: Spanned<Register>,
        lval: LValue,
    },
    Instr {
        mnemonic: Spanned<String>,
        args: Vec<Spanned<AsmOperand>>,
    },
    Error,
}

#[derive(Debug)]
pub enum AsmOperand {
    Reg(Register),
    Imm(u16),
    Error,
}
