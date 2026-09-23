use std::rc::Rc;

use crate::{
    CompilerCtx,
    ast::{AsmItem, AsmOperand, Expr, ExprKind, Item, ItemKind, LValue, Register, Stmt, StmtKind},
    diag::Diag,
    lexer::{Lexer, Token, TokenKind},
    op::{BinaryOp, UnaryOp},
    source::Spanned,
    ty::Ty,
};

macro_rules! expect_token {
    (
        $parser:expr,
        $token:expr,
        $pattern:pat $(if $pattern_cond:expr)? => $then:expr,
        $error:pat => $fallback:expr,
        Message: $message:literal
    ) => {
        match $token.inner {
            $pattern $(if $pattern_cond)? => $then,
            $error => $fallback,
            _ => {
                $parser.ctx.emit_diag(
                    Diag::new(&$parser.ctx.source, $message).with_label($token.span, true),
                );
                $fallback
            }
        }
    };
}

pub struct Parser {
    pub ctx: Rc<CompilerCtx>,
    lexer: Lexer,
    current: Token,
}

impl Parser {
    pub fn new(mut lexer: Lexer) -> Self {
        let current = lexer.next_token();
        Self {
            ctx: Rc::clone(&lexer.ctx),
            lexer,
            current,
        }
    }

    fn bump(&mut self) -> Token {
        let next = self.lexer.next_token();
        std::mem::replace(&mut self.current, next)
    }

    fn parse_int(&mut self) -> Expr {
        let tok = self.bump();
        expect_token!(
            self,
            tok,
            TokenKind::IntLit(v) => Expr::new(ExprKind::IntLit(v), tok.span),
            TokenKind::Error => Expr::new(ExprKind::Error, tok.span),
            Message: "expected an integer literal"
        )
    }

    fn parse_char(&mut self) -> Expr {
        let tok = self.bump();
        expect_token!(
            self,
            tok,
            TokenKind::CharLit(v) => Expr::new(ExprKind::CharLit(v), tok.span),
            TokenKind::Error => Expr::new(ExprKind::Error, tok.span),
            Message: "expected a character literal"
        )
    }

    fn parse_str(&mut self) -> Expr {
        let tok = self.bump();
        expect_token!(
            self,
            tok,
            TokenKind::StrLit(v) => Expr::new(ExprKind::StrLit(v), tok.span),
            TokenKind::Error => Expr::new(ExprKind::Error, tok.span),
            Message: "expected a string literal"
        )
    }

    fn parse_ident_or_path(&mut self) -> Expr {
        let tok = self.bump();
        let TokenKind::Ident(first) = tok.inner else {
            if !matches!(tok.inner, TokenKind::Error) {
                self.ctx.emit_diag(
                    Diag::new(&self.ctx.source, "expected an identifier")
                        .with_label(tok.span, true),
                );
            }
            return Expr::new(ExprKind::Error, tok.span);
        };

        if !matches!(self.current.inner, TokenKind::ColonColon) {
            return Expr::new(ExprKind::LValue(Box::new(LValue::Var(first))), tok.span);
        }

        let mut segments = vec![Spanned::new(first, tok.span)];
        let mut last_span = tok.span;
        while matches!(self.current.inner, TokenKind::ColonColon) {
            self.bump(); // ::
            let seg_tok = self.bump();
            let seg = expect_token!(
                self, seg_tok,
                TokenKind::Ident(i) => Some(i),
                TokenKind::Error => None,
                Message: "expected an identifier after a double colon"
            );
            last_span = seg_tok.span;
            match seg {
                Some(s) => segments.push(Spanned::new(s, seg_tok.span)),
                None => break,
            }
        }

        let span = tok.span.with_hi(last_span.hi);
        Expr::new(ExprKind::Path(segments), span)
    }

    fn parse_grouped(&mut self) -> Expr {
        let start = self.bump();
        expect_token!(
             self,
             start,
             TokenKind::LParen => {},
             TokenKind::Error => {},
             Message: "expected an opening parenthesis"
        );

        let expr = self.parse_expr();

        let end = self.bump();
        expect_token!(
            self,
            end,
            TokenKind::RParen => {},
            TokenKind::Error => {},
            Message: "expected a closing parenthesis"
        );

        Expr::new(expr.inner, start.span.with_hi(end.span.hi))
    }

    fn parse_atom(&mut self) -> Expr {
        match self.current.inner {
            TokenKind::IntLit(_) => self.parse_int(),
            TokenKind::CharLit(_) => self.parse_char(),
            TokenKind::StrLit(_) => self.parse_str(),
            TokenKind::Ident(_) => self.parse_ident_or_path(),
            TokenKind::LParen => self.parse_grouped(),
            TokenKind::Error => {
                let err = self.bump();
                Expr::new(ExprKind::Error, err.span)
            }
            _ => {
                let err = self.bump();
                self.ctx.emit_diag(
                    Diag::new(&self.ctx.source, "unexpected token").with_label(err.span, true),
                );
                Expr::new(ExprKind::Error, err.span)
            }
        }
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_atom();

        loop {
            if !matches!(self.current.inner, TokenKind::LParen) {
                break;
            }

            self.bump(); // (

            let mut params = Vec::new();

            if !matches!(self.current.inner, TokenKind::RParen) {
                loop {
                    params.push(self.parse_expr());

                    if matches!(self.current.inner, TokenKind::Comma) {
                        self.bump();
                    } else {
                        break;
                    }
                }
            }

            let end = self.bump();

            expect_token!(
                self,
                end,
                TokenKind::RParen => {},
                TokenKind::Error => {},
                Message: "expected a closing parenthesis"
            );

            let span = expr.span.with_hi(end.span.hi);
            expr = Expr::new(
                ExprKind::Call {
                    callee: Box::new(expr),
                    params,
                },
                span,
            );
        }

        expr
    }

    fn parse_lvalue(&mut self) -> Spanned<LValue> {
        match self.current.inner {
            TokenKind::Ident(_) => {
                let var = self.bump();
                let TokenKind::Ident(i) = var.inner else {
                    unreachable!()
                };
                Spanned::new(LValue::Var(i), var.span)
            }
            TokenKind::Star => {
                let star_span = self.bump().span; // *
                let expr = self.parse_postfix();
                let span = star_span.with_hi(expr.span.hi);
                Spanned::new(LValue::Deref(expr), span)
            }
            TokenKind::Error => {
                let error_span = self.bump().span;
                Spanned::new(LValue::Error, error_span)
            }
            _ => {
                let other = self.bump();
                self.ctx.emit_diag(
                    Diag::new(&self.ctx.source, "expected an l-value").with_label(other.span, true),
                );
                Spanned::new(LValue::Error, other.span)
            }
        }
    }

    fn parse_prefix(&mut self) -> Expr {
        match self.current.inner {
            TokenKind::Minus | TokenKind::Bang | TokenKind::Tilde => {
                let op = match self.current.inner {
                    TokenKind::Minus => UnaryOp::Neg,
                    TokenKind::Bang => UnaryOp::LogNot,
                    TokenKind::Tilde => UnaryOp::BitNot,
                    _ => unreachable!(),
                };
                let op_span = self.bump().span;
                let expr = Box::new(self.parse_prefix());
                let span = op_span.with_hi(expr.span.hi);
                Expr::new(ExprKind::Unary { op, expr }, span)
            }
            TokenKind::Amp => {
                let op_span = self.bump().span;
                let expr = self.parse_lvalue().map(Box::new);
                let span = op_span.with_hi(expr.span.hi);
                Expr::new(ExprKind::Ref(expr), span)
            }
            _ => self.parse_postfix(),
        }
    }

    fn binary_op(&self) -> Option<(BinaryOp, u8)> {
        match self.current.inner {
            TokenKind::Plus => Some((BinaryOp::Add, 10)),
            TokenKind::Minus => Some((BinaryOp::Sub, 10)),
            TokenKind::Star => Some((BinaryOp::Mul, 11)),
            TokenKind::Slash => Some((BinaryOp::Div, 11)),
            TokenKind::Percent => Some((BinaryOp::Mod, 11)),
            TokenKind::Lt => Some((BinaryOp::Lt, 8)),
            TokenKind::Le => Some((BinaryOp::Le, 8)),
            TokenKind::Gt => Some((BinaryOp::Gt, 8)),
            TokenKind::Ge => Some((BinaryOp::Ge, 8)),
            TokenKind::EqEq => Some((BinaryOp::Eq, 7)),
            TokenKind::BangEq => Some((BinaryOp::Ne, 7)),
            TokenKind::Amp => Some((BinaryOp::BitAnd, 6)),
            TokenKind::Caret => Some((BinaryOp::BitXor, 5)),
            TokenKind::Pipe => Some((BinaryOp::BitOr, 4)),
            TokenKind::AmpAmp => Some((BinaryOp::LogAnd, 3)),
            TokenKind::PipePipe => Some((BinaryOp::LogOr, 2)),
            _ => None,
        }
    }

    fn parse_binary(&mut self, min_prec: u8) -> Expr {
        let mut left = self.parse_prefix();

        loop {
            let Some((op, prec)) = self.binary_op() else {
                break;
            };

            if prec < min_prec {
                break;
            }

            self.bump();
            let right = self.parse_binary(prec + 1);

            let span = left.span.with_hi(right.span.hi);

            left = Expr::new(
                ExprKind::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    fn parse_ty(&mut self) -> Ty {
        match self.current.inner {
            TokenKind::Ident(ref name) if name == "int" => {
                self.bump();
                Ty::Int
            }
            TokenKind::Ident(ref name) if name == "func" => {
                self.bump();
                let lparen = self.bump();
                expect_token!(
                    self,
                    lparen,
                    TokenKind::LParen => {},
                    TokenKind::Error => {},
                    Message: "expected an opening parenthesis"
                );

                let mut params = Vec::new();
                while !matches!(self.current.inner, TokenKind::RParen) {
                    let ty = self.parse_ty();
                    params.push(ty);

                    if matches!(self.current.inner, TokenKind::Comma) {
                        self.bump();
                    } else {
                        break;
                    }
                }

                let rparen = self.bump();
                expect_token!(
                    self,
                    rparen,
                    TokenKind::RParen => {},
                    TokenKind::Error => {},
                    Message: "expected a closing parenthesis"
                );

                let ret = match self.current.inner {
                    TokenKind::Colon => {
                        self.bump();
                        self.parse_ty()
                    }
                    _ => Ty::Unit,
                };
                Ty::Func(params, Box::new(ret))
            }
            TokenKind::Ident(ref name) if name == "unit" => {
                self.bump();
                Ty::Unit
            }
            TokenKind::Star => {
                self.bump();
                Ty::Ptr(Box::new(self.parse_ty()))
            }
            _ => {
                let tok = self.bump();
                self.ctx.emit_diag(
                    Diag::new(&self.ctx.source, "expected a type").with_label(tok.span, true),
                );
                Ty::Unit
            }
        }
    }

    fn parse_expr(&mut self) -> Expr {
        let left = self.parse_binary(0);

        if !matches!(self.current.inner, TokenKind::Eq) {
            return left;
        }

        self.bump(); // =

        let Expr {
            inner: ExprKind::LValue(lval),
            span: left_span,
        } = left
        else {
            self.ctx.emit_diag(
                Diag::new(&self.ctx.source, "expected an l-value").with_label(left.span, true),
            );

            let rhs = self.parse_expr();
            return Expr::new(ExprKind::Error, rhs.span);
        };

        let lval = Spanned::new(lval, left_span);

        let rhs = self.parse_expr();
        let span = left_span.with_hi(rhs.span.hi);
        Expr::new(
            ExprKind::Assign {
                lval,
                expr: Box::new(rhs),
            },
            span,
        )
    }

    fn parse_let(&mut self) -> Stmt {
        let start = self.bump();
        expect_token!(
            self,
            start,
            TokenKind::Ident(kw) if kw == "let" => {},
            TokenKind::Error => {},
            Message: "expected a let keyword"
        );

        let ident_tok = self.bump();
        let ident = expect_token!(
            self,
            ident_tok,
            TokenKind::Ident(i) => Some(i),
            TokenKind::Error => None,
            Message: "expected an identifier"
        );

        let ty = if matches!(self.current.inner, TokenKind::Colon) {
            self.bump(); // :
            Some(self.parse_ty())
        } else {
            None
        };

        let val = if matches!(self.current.inner, TokenKind::Eq) {
            self.bump(); // =
            Some(self.parse_expr())
        } else {
            None
        };

        let end = self.bump();
        expect_token!(
            self,
            end,
            TokenKind::Semicolon => {},
            TokenKind::Error => {},
            Message: "expected a semicolon"
        );

        if let Some(ident) = ident {
            Stmt::new(
                StmtKind::Let {
                    name: Spanned::new(ident, ident_tok.span),
                    ty,
                    val,
                },
                start.span.with_hi(end.span.hi),
            )
        } else {
            Stmt::new(StmtKind::Error, start.span.with_hi(end.span.hi))
        }
    }

    fn parse_block(&mut self) -> Stmt {
        let start = self.bump();

        expect_token!(
            self,
            start,
            TokenKind::LBrace => {},
            TokenKind::Error => {},
            Message: "expected an opening brace"
        );

        let mut stmts = Vec::new();

        while !matches!(self.current.inner, TokenKind::RBrace | TokenKind::Eof) {
            stmts.push(self.parse_stmt());
        }

        let end = self.bump();

        expect_token!(
            self,
            end,
            TokenKind::RBrace => {},
            TokenKind::Error => {},
            Message: "expected a closing brace"
        );

        Stmt::new(StmtKind::Block(stmts), start.span.with_hi(end.span.hi))
    }

    fn parse_if(&mut self) -> Stmt {
        let start = self.bump();
        expect_token!(
            self,
            start,
            TokenKind::Ident(kw) if kw == "if" => {},
            TokenKind::Error => {},
            Message: "expected an if keyword"
        );

        let cond = self.parse_expr();
        let then_branch = Box::new(self.parse_stmt());

        let else_branch = if matches!(
            self.current.inner,
            TokenKind::Ident(ref kw) if kw == "else"
        ) {
            self.bump(); // else
            Some(Box::new(self.parse_stmt()))
        } else {
            None
        };

        let end_span = else_branch
            .as_ref()
            .map(|stmt| stmt.span)
            .unwrap_or(then_branch.span);

        Stmt::new(
            StmtKind::If {
                cond,
                then_branch,
                else_branch,
            },
            start.span.with_hi(end_span.hi),
        )
    }

    fn parse_while(&mut self) -> Stmt {
        let start = self.bump();
        expect_token!(
            self,
            start,
            TokenKind::Ident(kw) if kw == "while" => {},
            TokenKind::Error => {},
            Message: "expected a while keyword"
        );

        let cond = self.parse_expr();
        let body = Box::new(self.parse_stmt());

        let span = start.span.with_hi(body.span.hi);
        Stmt::new(StmtKind::While { cond, body }, span)
    }

    fn parse_expr_stmt(&mut self) -> Stmt {
        let expr = self.parse_expr();
        let semi = self.bump();
        expect_token!(
            self,
            semi,
            TokenKind::Semicolon => {
                let span = expr.span.with_hi(semi.span.hi);
                Stmt::new(StmtKind::Expr(expr), span)
            },
            TokenKind::Error => Stmt::new(StmtKind::Error, semi.span),
            Message: "expected a semicolon"
        )
    }

    fn parse_asm_operand(&mut self) -> Spanned<AsmOperand> {
        match &self.current.inner {
            TokenKind::Ident(name) if Register::from_name(name).is_some() => {
                let reg = Register::from_name(name).unwrap();
                let tok = self.bump();
                Spanned::new(AsmOperand::Reg(reg), tok.span)
            }
            TokenKind::IntLit(_) => {
                let tok = self.bump();
                let TokenKind::IntLit(n) = tok.inner else {
                    unreachable!()
                };
                Spanned::new(AsmOperand::Imm(n), tok.span)
            }
            TokenKind::Error => {
                let tok = self.bump();
                Spanned::new(AsmOperand::Imm(0), tok.span)
            }
            _ => {
                let tok = self.bump();
                self.ctx.emit_diag(
                    Diag::new(&self.ctx.source, "expected a register or an immediate")
                        .with_label(tok.span, true),
                );
                Spanned::new(AsmOperand::Error, tok.span)
            }
        }
    }

    fn parse_asm_item(&mut self) -> Spanned<AsmItem> {
        let name_tok = self.bump();
        let name_span = name_tok.span;

        let TokenKind::Ident(name) = name_tok.inner else {
            if !matches!(name_tok.inner, TokenKind::Error) {
                self.ctx.emit_diag(
                    Diag::new(
                        &self.ctx.source,
                        "expected a register name or an instruction mnemonic",
                    )
                    .with_label(name_span, true),
                );
            }
            return Spanned::new(
                AsmItem::Instr {
                    mnemonic: Spanned::new(String::new(), name_span),
                    args: vec![],
                },
                name_span,
            );
        };

        // `regname = in(...)` or `regname = out(...)`
        if let Some(reg) = Register::from_name(&name) {
            if matches!(self.current.inner, TokenKind::Eq) {
                self.bump(); // =
                let dir_tok = self.bump();
                let dir = expect_token!(
                    self, dir_tok,
                    TokenKind::Ident(d) => Some(d),
                    TokenKind::Error => None,
                    Message: "expected `in` or `out`"
                );

                let lparen = self.bump();
                expect_token!(
                    self,
                    lparen,
                    TokenKind::LParen => {},
                    TokenKind::Error => {},
                    Message: "expected an opening parenthesis"
                );

                let item = match dir.as_deref() {
                    Some("in") => {
                        let expr = self.parse_expr();
                        AsmItem::In {
                            reg: Spanned::new(reg, name_span),
                            expr,
                        }
                    }
                    Some("out") => {
                        let lval = self.parse_lvalue();
                        AsmItem::Out {
                            reg: Spanned::new(reg, name_span),
                            lval: lval.inner,
                        }
                    }
                    _ => {
                        if dir.is_some() {
                            self.ctx.emit_diag(
                                Diag::new(&self.ctx.source, "expected `in` or `out`")
                                    .with_label(dir_tok.span, true),
                            );
                        }
                        let _ = self.parse_expr();
                        AsmItem::Error
                    }
                };

                let rparen = self.bump();
                expect_token!(
                    self,
                    rparen,
                    TokenKind::RParen => {},
                    TokenKind::Error => {},
                    Message: "expected a closing parenthesis"
                );

                return Spanned::new(item, name_span.with_hi(rparen.span.hi));
            }
        }

        let mnemonic = Spanned::new(name, name_span);
        let mut args = Vec::new();
        let mut last_hi = name_span.hi;
        while !matches!(
            self.current.inner,
            TokenKind::Comma | TokenKind::RBrace | TokenKind::Eof
        ) {
            let operand = self.parse_asm_operand();
            last_hi = operand.span.hi;
            args.push(operand);
        }

        Spanned::new(
            AsmItem::Instr { mnemonic, args },
            name_span.with_hi(last_hi),
        )
    }

    fn parse_asm(&mut self) -> Stmt {
        let start = self.bump();
        expect_token!(
            self,
            start,
            TokenKind::Ident(kw) if kw == "asm" => {},
            TokenKind::Error => {},
            Message: "expected an asm keyword"
        );

        let lbrace = self.bump();
        expect_token!(
            self,
            lbrace,
            TokenKind::LBrace => {},
            TokenKind::Error => {},
            Message: "expected an opening brace"
        );

        let mut asm_items = Vec::new();

        while !matches!(self.current.inner, TokenKind::RBrace) {
            let item = self.parse_asm_item();
            asm_items.push(item);

            if matches!(self.current.inner, TokenKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }

        let end = self.bump();
        expect_token!(
            self,
            end,
            TokenKind::RBrace => {},
            TokenKind::Error => {},
            Message: "expected a closing brace"
        );

        Stmt::new(StmtKind::Asm(asm_items), start.span.with_hi(end.span.hi))
    }

    fn parse_return(&mut self) -> Stmt {
        let start = self.bump();
        expect_token!(
            self,
            start,
            TokenKind::Ident(kw) if kw == "return" => {},
            TokenKind::Error => {},
            Message: "expected a return keyword"
        );
        match self.current.inner {
            TokenKind::Semicolon => {
                let end = self.bump();
                Stmt::new(StmtKind::Return(None), start.span.with_hi(end.span.hi))
            }
            _ => {
                let expr = self.parse_expr();
                let end = self.bump();
                expect_token!(
                    self,
                    end,
                    TokenKind::Semicolon => {
                        let span = expr.span.with_hi(end.span.hi);
                        Stmt::new(StmtKind::Return(Some(expr)), span)
                    },
                    TokenKind::Error => Stmt::new(StmtKind::Error, end.span),
                    Message: "expected a semicolon"
                )
            }
        }
    }

    fn parse_stmt(&mut self) -> Stmt {
        match &self.current.inner {
            TokenKind::Ident(i) if i == "let" => self.parse_let(),
            TokenKind::LBrace => self.parse_block(),
            TokenKind::Ident(i) if i == "if" => self.parse_if(),
            TokenKind::Ident(i) if i == "while" => self.parse_while(),
            TokenKind::Ident(i) if i == "asm" => self.parse_asm(),
            TokenKind::Ident(i) if i == "return" => self.parse_return(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_import(&mut self) -> Item {
        let start = self.bump();
        expect_token!(
            self,
            start,
            TokenKind::Ident(kw) if kw == "import" => {},
            TokenKind::Error => {},
            Message: "expected an import keyword"
        );

        let name_tok = self.bump();
        let name = expect_token!(
            self, name_tok,
            TokenKind::Ident(i) => Some(Spanned::new(i, name_tok.span)),
            TokenKind::Error => None,
            Message: "expected a module name"
        );

        let end = self.bump();
        expect_token!(
            self,
            end,
            TokenKind::Semicolon => {},
            TokenKind::Error => {},
            Message: "expected a semicolon"
        );

        if let Some(name) = name {
            Item::new(ItemKind::Import(name), start.span.with_hi(end.span.hi))
        } else {
            Item::new(ItemKind::Error, start.span.with_hi(end.span.hi))
        }
    }

    fn parse_func(&mut self) -> Item {
        let start = self.bump();
        expect_token!(
            self,
            start,
            TokenKind::Ident(kw) if kw == "func" => {},
            TokenKind::Error => {},
            Message: "expected a func keyword"
        );

        let is_extern = match &self.current.inner {
            TokenKind::Ident(kw) if kw == "ext" => {
                self.bump();
                true
            }
            _ => false,
        };

        let name_tok = self.bump();
        let name = expect_token!(
            self,
            name_tok,
            TokenKind::Ident(i) => Some(i),
            TokenKind::Error => None,
            Message: "expected an identifier"
        );

        let lparen = self.bump();
        expect_token!(
            self,
            lparen,
            TokenKind::LParen => {},
            TokenKind::Error => {},
            Message: "expected an opening parenthesis"
        );

        if is_extern {
            let mut params = Vec::new();
            while !matches!(self.current.inner, TokenKind::RParen) {
                let ty = self.parse_ty();
                params.push(ty);

                if matches!(self.current.inner, TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }

            let rparen = self.bump();
            expect_token!(
                self,
                rparen,
                TokenKind::RParen => {},
                TokenKind::Error => {},
                Message: "expected a closing parenthesis"
            );

            let ret = match self.current.inner {
                TokenKind::Colon => {
                    self.bump();
                    self.parse_ty()
                }
                _ => Ty::Unit,
            };

            let end = self.bump();
            expect_token!(
                self,
                end,
                TokenKind::Semicolon => {},
                TokenKind::Error => {},
                Message: "expected a semicolon"
            );

            if let Some(name) = name {
                Item::new(
                    ItemKind::FunctionExternal {
                        name: Spanned::new(name, name_tok.span),
                        params,
                        ret,
                    },
                    start.span.with_hi(end.span.hi),
                )
            } else {
                Item::new(ItemKind::Error, start.span.with_hi(end.span.hi))
            }
        } else {
            let mut params = Vec::new();
            while !matches!(self.current.inner, TokenKind::RParen) {
                let name_tok = self.bump();
                let name = expect_token!(
                    self,
                    name_tok,
                    TokenKind::Ident(i) => Some(Spanned::new(i, name_tok.span)),
                    TokenKind::Error => None,
                    Message: "expected an identifier"
                );

                let colon = self.bump();
                expect_token!(
                    self,
                    colon,
                    TokenKind::Colon => {},
                    TokenKind::Error => {},
                    Message: "expected a colon"
                );

                let ty = self.parse_ty();
                params.push((name, ty));

                if matches!(self.current.inner, TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }

            let rparen = self.bump();
            expect_token!(
                self,
                rparen,
                TokenKind::RParen => {},
                TokenKind::Error => {},
                Message: "expected a closing parenthesis"
            );

            let ret = match self.current.inner {
                TokenKind::Colon => {
                    self.bump();
                    self.parse_ty()
                }
                _ => Ty::Unit,
            };

            let body = self.parse_stmt();
            let span = start.span.with_hi(body.span.hi);
            if let Some(name) = name {
                let mut params2 = Vec::new();
                for param in params {
                    if let (Some(name), ty) = param {
                        params2.push((name, ty));
                    } else {
                        return Item::new(ItemKind::Error, span);
                    }
                }

                Item::new(
                    ItemKind::FunctionDefined {
                        name: Spanned::new(name, name_tok.span),
                        params: params2,
                        ret,
                        body,
                    },
                    span,
                )
            } else {
                Item::new(ItemKind::Error, span)
            }
        }
    }

    fn parse_item(&mut self) -> Item {
        match &self.current.inner {
            TokenKind::Ident(kw) if kw == "import" => self.parse_import(),
            TokenKind::Ident(kw) if kw == "func" => self.parse_func(),
            TokenKind::Error => {
                let tok = self.bump();
                Item::new(ItemKind::Error, tok.span)
            }
            _ => {
                let tok = self.bump();
                self.ctx.emit_diag(
                    Diag::new(&self.ctx.source, "expected an item").with_label(tok.span, true),
                );
                Item::new(ItemKind::Error, tok.span)
            }
        }
    }

    pub fn maybe_item(&mut self) -> Option<Item> {
        match &self.current.inner {
            TokenKind::Eof => None,
            _ => Some(self.parse_item()),
        }
    }
}
