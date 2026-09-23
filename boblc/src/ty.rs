#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Int,
    Ptr(Box<Ty>),
    Func(Vec<Ty>, Box<Ty>),
    Unit,
}

impl std::fmt::Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int => write!(f, "int"),
            Self::Ptr(inner) => write!(f, "*{inner}"),
            Self::Func(params, ret) => write!(
                f,
                "func({}): {ret}",
                params
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Unit => write!(f, "unit"),
        }
    }
}
