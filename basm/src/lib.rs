pub mod lexer;
pub mod parser;
pub mod pass1;
pub mod pass2;

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub value: T,
    pub line: usize,
}
