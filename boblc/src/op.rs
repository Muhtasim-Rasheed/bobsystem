use crate::ty::Ty;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    LogAnd,
    LogOr,
    BitAnd,
    BitOr,
    BitXor,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl BinaryOp {
    pub fn res_ty(self, lhs: &Ty, rhs: &Ty) -> Option<Ty> {
        use BinaryOp::*;
        use Ty::*;

        if lhs == &Unit || rhs == &Unit {
            return None;
        }

        match self {
            Add | Sub | Mul | Div | Mod | LogAnd | LogOr | BitAnd | BitOr | BitXor | Lt | Le
            | Gt | Ge => ((lhs, rhs) == (&Int, &Int)).then_some(Int),
            Eq | Ne => (lhs == rhs).then_some(Int),
        }
    }
}

impl std::fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
            Self::Mod => write!(f, "%"),
            Self::LogAnd => write!(f, "&&"),
            Self::LogOr => write!(f, "||"),
            Self::BitAnd => write!(f, "&"),
            Self::BitOr => write!(f, "|"),
            Self::BitXor => write!(f, "^"),
            Self::Eq => write!(f, "=="),
            Self::Ne => write!(f, "!="),
            Self::Lt => write!(f, "<"),
            Self::Le => write!(f, "<="),
            Self::Gt => write!(f, ">"),
            Self::Ge => write!(f, ">="),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    BitNot,
    LogNot,
    Neg,
}

impl UnaryOp {
    pub fn res_ty(self, ty: &Ty) -> Option<Ty> {
        use Ty::*;

        match (self, ty) {
            (_, Unit) => None,
            (UnaryOp::BitNot, Int) => Some(Int),
            (UnaryOp::LogNot, Int) => Some(Int),
            (UnaryOp::Neg, Int) => Some(Int),
            _ => None,
        }
    }
}

impl std::fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BitNot => write!(f, "~"),
            Self::LogNot => write!(f, "!"),
            Self::Neg => write!(f, "-"),
        }
    }
}
