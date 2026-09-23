use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    CompilerCtx,
    ast::{
        AsmItem, AsmOperand, Expr, ExprKind, ItemKind, LValue, Program, Register, Stmt, StmtKind,
    },
    diag::Diag,
    rast::{
        FuncId, ModuleId, RAsmItem, RAsmOperand, RExpr, RExprKind, RFunction, RLValue, RModule,
        RRegister, RStmt, VarId,
    },
    source::{Span, Spanned},
    ty::Ty,
};

#[derive(Clone)]
struct VarInfo {
    id: VarId,
    ty: Ty,
}

#[derive(Clone)]
struct FuncInfo {
    id: FuncId,
    ty: Ty, // always Ty::Func(params, ret)
}

#[derive(Default)]
struct ResolverIds {
    next_func: u32,
    next_var: u32,
}

pub struct Resolver {
    pub ctx: Rc<CompilerCtx>,
    ids: Rc<RefCell<ResolverIds>>,
    modules: Vec<RModule>,

    module_names: HashMap<String, ModuleId>,
    functions: HashMap<ModuleId, HashMap<String, FuncInfo>>,
    module_imports: HashMap<ModuleId, Vec<ModuleId>>,

    current_module: ModuleId,
    current_locals: Vec<Ty>,
    scopes: Vec<HashMap<String, VarInfo>>,
}

impl Resolver {
    pub fn new(ctx: Rc<CompilerCtx>) -> Self {
        Self {
            ctx,
            ids: Rc::default(),
            modules: Vec::new(),
            module_names: HashMap::new(),
            functions: HashMap::new(),
            module_imports: HashMap::new(),
            current_module: ModuleId(0),
            current_locals: Vec::new(),
            scopes: Vec::new(),
        }
    }

    pub fn register_module(&mut self, name: &str, module_id: ModuleId, program: &Program) {
        self.module_names.insert(name.to_string(), module_id);
        let mut fns = HashMap::new();

        for item in &program.items {
            match &item.inner {
                ItemKind::FunctionExternal { name, params, ret } => {
                    let id = self.new_func_id();
                    fns.insert(
                        name.inner.clone(),
                        FuncInfo {
                            id,
                            ty: Ty::Func(params.clone(), Box::new(ret.clone())),
                        },
                    );
                }
                ItemKind::FunctionDefined {
                    name, params, ret, ..
                } => {
                    let id = self.new_func_id();
                    let param_tys = params.iter().map(|(_, t)| t.clone()).collect();
                    fns.insert(
                        name.inner.clone(),
                        FuncInfo {
                            id,
                            ty: Ty::Func(param_tys, Box::new(ret.clone())),
                        },
                    );
                }
                ItemKind::Import(_) | ItemKind::Error => {}
            }
        }

        self.functions.insert(module_id, fns);
    }

    pub fn register_imports(&mut self, module_id: ModuleId, program: &Program) {
        let mut imports = Vec::new();
        for item in &program.items {
            if let ItemKind::Import(name) = &item.inner {
                match self.module_names.get(&name.inner) {
                    Some(&imported) => imports.push(imported),
                    None => {
                        self.ctx.emit_diag(
                            Diag::new(&self.ctx.source, format!("unknown module `{}`", name.inner))
                                .with_label(name.span, true),
                        );
                    }
                }
            }
        }
        self.module_imports.insert(module_id, imports);
    }

    pub fn resolve_module(&mut self, module_id: ModuleId, ctx: Rc<CompilerCtx>, program: Program) {
        self.ctx = ctx;
        self.current_module = module_id;
        let mut functions = Vec::new();

        for item in program.items {
            match item.inner {
                ItemKind::FunctionDefined {
                    name,
                    params,
                    ret,
                    body,
                } => {
                    if let Some(f) =
                        self.resolve_function(module_id, &name.inner, params, ret, body)
                    {
                        functions.push(f);
                    }
                }
                ItemKind::FunctionExternal { .. } | ItemKind::Import(_) | ItemKind::Error => {}
            }
        }

        self.modules.push(RModule {
            id: module_id,
            functions,
        });
    }

    fn resolve_function(
        &mut self,
        module_id: ModuleId,
        name: &str,
        params: Vec<(Spanned<String>, Ty)>,
        ret: Ty,
        body: Stmt,
    ) -> Option<RFunction> {
        let id = self.functions.get(&module_id)?.get(name)?.id;

        self.current_locals = Vec::new();
        self.scopes = vec![HashMap::new()];

        let mut rparams = Vec::new();
        for (pname, pty) in params {
            let var_id = self.new_var(pty.clone());
            self.scopes.last_mut().unwrap().insert(
                pname.inner,
                VarInfo {
                    id: var_id,
                    ty: pty.clone(),
                },
            );
            rparams.push((var_id, pty));
        }

        let rbody = self.resolve_stmt(body);

        Some(RFunction {
            id,
            module: module_id,
            name: name.to_string(),
            params: rparams,
            ret,
            locals: std::mem::take(&mut self.current_locals),
            body: rbody,
        })
    }

    pub fn into_program(self) -> crate::rast::RProgram {
        crate::rast::RProgram {
            modules: self.modules,
        }
    }

    fn new_func_id(&mut self) -> FuncId {
        let mut ids = self.ids.borrow_mut();
        let id = ids.next_func;
        ids.next_func += 1;
        FuncId(id)
    }

    fn new_var(&mut self, ty: Ty) -> VarId {
        let id = {
            let mut ids = self.ids.borrow_mut();
            let id = ids.next_var;
            ids.next_var += 1;
            id
        };
        self.current_locals.push(ty);
        VarId(id)
    }

    fn find_var(&self, name: &str) -> Option<VarInfo> {
        for i in (0..self.scopes.len()).rev() {
            if let Some(info) = self.scopes[i].get(name) {
                return Some(info.clone());
            }
        }
        None
    }

    fn find_func_in(&self, module: ModuleId, name: &str) -> Option<FuncInfo> {
        self.functions.get(&module)?.get(name).cloned()
    }

    fn resolve_name(&mut self, name: String, span: Span) -> RExpr {
        if let Some(info) = self.find_var(&name) {
            return RExpr {
                kind: RExprKind::LValue(RLValue::Var(info.id)),
                ty: info.ty,
            };
        }

        if let Some(func_info) = self.find_func_in(self.current_module, &name) {
            return RExpr {
                kind: RExprKind::Function(func_info.id),
                ty: func_info.ty,
            };
        }

        self.ctx.emit_diag(
            Diag::new(&self.ctx.source, format!("unknown name {}", name)).with_label(span, true),
        );

        err_rexpr()
    }

    fn resolve_lval(&mut self, lval: LValue, span: Span) -> (RLValue, Ty) {
        match lval {
            LValue::Var(name) => {
                if let Some(info) = self.find_var(&name) {
                    (RLValue::Var(info.id), info.ty)
                } else {
                    self.ctx.emit_diag(
                        Diag::new(&self.ctx.source, format!("unknown variable {name}"))
                            .with_label(span, true),
                    );
                    (RLValue::Error, Ty::Unit)
                }
            }
            LValue::Deref(inner_expr) => {
                let resolved = self.resolve_expr(inner_expr);
                if matches!(resolved.kind, RExprKind::Error) {
                    return (RLValue::Error, Ty::Unit);
                }
                if let Ty::Ptr(inner_ty) = resolved.ty.clone() {
                    (RLValue::Deref(Box::new(resolved)), *inner_ty)
                } else {
                    self.ctx.emit_diag(
                        Diag::new(&self.ctx.source, "dereference of a non-pointer")
                            .with_message_label(
                                format!("dereferenced a value with type {} here, which is not a pointer", resolved.ty),
                                span, true,
                            ),
                    );
                    (RLValue::Error, Ty::Unit)
                }
            }
            LValue::Error => (RLValue::Error, Ty::Unit),
        }
    }

    fn resolve_path(&mut self, path: Vec<Spanned<String>>) -> RExpr {
        if path.len() != 2 {
            let span = path.first().map(|s| s.span).unwrap_or(Span::new(0, 0));
            self.ctx.emit_diag(
                Diag::new(
                    &self.ctx.source,
                    "expected a path of the form `module::item`",
                )
                .with_label(span, true),
            );
            return err_rexpr();
        }

        let module_seg = &path[0];
        let item_seg = &path[1];

        let Some(&target_module) = self.module_names.get(&module_seg.inner) else {
            self.ctx.emit_diag(
                Diag::new(
                    &self.ctx.source,
                    format!("unknown module `{}`", module_seg.inner),
                )
                .with_label(module_seg.span, true),
            );
            return err_rexpr();
        };

        let imported = target_module == self.current_module
            || self
                .module_imports
                .get(&self.current_module)
                .is_some_and(|imports| imports.contains(&target_module));
        if !imported {
            self.ctx.emit_diag(
                Diag::new(
                    &self.ctx.source,
                    format!("module `{}` was not imported", module_seg.inner),
                )
                .with_message_label(
                    "add `import` for this module before using it",
                    module_seg.span,
                    true,
                ),
            );
            return err_rexpr();
        }

        let Some(func_info) = self.find_func_in(target_module, &item_seg.inner) else {
            self.ctx.emit_diag(
                Diag::new(
                    &self.ctx.source,
                    format!(
                        "no function `{}` in module `{}`",
                        item_seg.inner, module_seg.inner
                    ),
                )
                .with_label(item_seg.span, true),
            );
            return err_rexpr();
        };

        RExpr {
            kind: RExprKind::Function(func_info.id),
            ty: func_info.ty,
        }
    }

    fn resolve_expr(&mut self, expr: Expr) -> RExpr {
        match expr.inner {
            ExprKind::IntLit(i) => RExpr {
                kind: RExprKind::IntLit(i),
                ty: Ty::Int,
            },
            ExprKind::CharLit(i) => RExpr {
                kind: RExprKind::CharLit(i),
                ty: Ty::Int,
            },
            ExprKind::StrLit(i) => RExpr {
                kind: RExprKind::StrLit(i),
                ty: Ty::Ptr(Box::new(Ty::Int)),
            },
            ExprKind::LValue(lval) => match *lval {
                LValue::Var(name) => self.resolve_name(name, expr.span),
                other => {
                    let (rlval, ty) = self.resolve_lval(other, expr.span);

                    if matches!(rlval, RLValue::Error) {
                        return err_rexpr();
                    }

                    RExpr {
                        kind: RExprKind::LValue(rlval),
                        ty,
                    }
                }
            },
            ExprKind::Ref(inner_lval) => {
                let (resolved, ty) = self.resolve_lval(*inner_lval.inner, inner_lval.span);
                if matches!(resolved, RLValue::Error) {
                    return err_rexpr();
                }
                let ty = Ty::Ptr(Box::new(ty));
                RExpr {
                    kind: RExprKind::Ref(resolved),
                    ty,
                }
            }
            ExprKind::Path(path) => self.resolve_path(path),
            ExprKind::Binary { left, op, right } => {
                let rleft = self.resolve_expr(*left);
                let rright = self.resolve_expr(*right);
                if matches!(rleft.kind, RExprKind::Error) || matches!(rright.kind, RExprKind::Error)
                {
                    return err_rexpr();
                }
                if let Some(ty) = op.res_ty(&rleft.ty, &rright.ty) {
                    RExpr {
                        kind: RExprKind::Binary {
                            left: Box::new(rleft),
                            op,
                            right: Box::new(rright),
                        },
                        ty,
                    }
                } else {
                    self.ctx.emit_diag(
                        Diag::new(&self.ctx.source, "incompatible types in binary expression")
                            .with_message_label(
                                format!(
                                    "{} cannot be used with {} and {}",
                                    op, rleft.ty, rright.ty
                                ),
                                expr.span,
                                true,
                            ),
                    );
                    err_rexpr()
                }
            }
            ExprKind::Unary {
                op,
                expr: expr_inner,
            } => {
                let rexpr = self.resolve_expr(*expr_inner);
                if matches!(rexpr.kind, RExprKind::Error) {
                    return rexpr;
                }
                if let Some(ty) = op.res_ty(&rexpr.ty) {
                    RExpr {
                        kind: RExprKind::Unary {
                            op,
                            expr: Box::new(rexpr),
                        },
                        ty,
                    }
                } else {
                    self.ctx.emit_diag(
                        Diag::new(&self.ctx.source, "incompatible type in unary expression")
                            .with_message_label(
                                format!("{op} cannot be used with {}", rexpr.ty),
                                expr.span,
                                true,
                            ),
                    );
                    err_rexpr()
                }
            }
            ExprKind::Assign {
                lval,
                expr: val_expr,
            } => {
                let (rlval, rlval_ty) = self.resolve_lval(*lval.inner, lval.span);
                let val_rexpr = self.resolve_expr(*val_expr);
                if matches!(rlval, RLValue::Error) {
                    return err_rexpr();
                }
                if matches!(val_rexpr.kind, RExprKind::Error) {
                    return val_rexpr;
                }
                if val_rexpr.ty == rlval_ty {
                    RExpr {
                        kind: RExprKind::Assign {
                            target: rlval,
                            val: Box::new(val_rexpr),
                        },
                        ty: rlval_ty,
                    }
                } else {
                    self.ctx.emit_diag(
                        Diag::new(&self.ctx.source, "type mismatch").with_message_label(
                            format!(
                                "cannot assign value of type {} to place of type {}",
                                val_rexpr.ty, rlval_ty
                            ),
                            expr.span,
                            true,
                        ),
                    );
                    err_rexpr()
                }
            }
            ExprKind::Call { callee, params } => {
                let callee_span = callee.span;
                let rcallee = self.resolve_expr(*callee);
                if matches!(rcallee.kind, RExprKind::Error) {
                    return rcallee;
                }

                let args: Vec<_> = params.into_iter().map(|p| self.resolve_expr(p)).collect();
                if args.iter().any(|a| matches!(a.kind, RExprKind::Error)) {
                    return err_rexpr();
                }

                if let Ty::Func(param_tys, ret_ty) = rcallee.ty.clone() {
                    let mut valid = true;
                    if args.len() != param_tys.len() {
                        self.ctx.emit_diag(
                            Diag::new(&self.ctx.source, "incorrect number of arguments")
                                .with_message_label(
                                    format!(
                                        "expected {} arguments, got {}",
                                        param_tys.len(),
                                        args.len()
                                    ),
                                    expr.span,
                                    true,
                                ),
                        );
                        valid = false;
                    }
                    for (i, (arg, expected)) in args.iter().zip(param_tys.iter()).enumerate() {
                        if &arg.ty != expected {
                            self.ctx.emit_diag(
                                Diag::new(&self.ctx.source, "type mismatch").with_message_label(
                                    format!(
                                        "argument {} has type {}, expected {}",
                                        i + 1,
                                        arg.ty,
                                        expected
                                    ),
                                    expr.span,
                                    true,
                                ),
                            );
                            valid = false;
                        }
                    }
                    if !valid {
                        return err_rexpr();
                    }
                    RExpr {
                        kind: RExprKind::Call {
                            callee: Box::new(rcallee),
                            args,
                        },
                        ty: *ret_ty,
                    }
                } else {
                    self.ctx.emit_diag(
                        Diag::new(&self.ctx.source, "calling an un-callable value")
                            .with_message_label(
                                format!("this has type {} which is not callable", rcallee.ty),
                                callee_span,
                                true,
                            ),
                    );
                    err_rexpr()
                }
            }
            ExprKind::Error => err_rexpr(),
        }
    }

    fn resolve_stmt(&mut self, stmt: Stmt) -> RStmt {
        match stmt.inner {
            StmtKind::Let { name, ty, val } => {
                let rval = val.map(|v| self.resolve_expr(v));

                let final_ty = match (&ty, &rval) {
                    (Some(t), Some(v)) => {
                        if !matches!(v.kind, RExprKind::Error) && &v.ty != t {
                            self.ctx.emit_diag(
                                Diag::new(&self.ctx.source, "type mismatch in let binding")
                                    .with_message_label(
                                        format!("expected {t}, found {}", v.ty),
                                        name.span,
                                        true,
                                    ),
                            );
                        }
                        t.clone()
                    }
                    (Some(t), None) => t.clone(),
                    (None, Some(v)) => v.ty.clone(),
                    (None, None) => {
                        self.ctx.emit_diag(
                            Diag::new(&self.ctx.source, "cannot infer the type of this variable")
                                .with_message_label(
                                    "add a type annotation (`: TYPE`) or an initializer",
                                    name.span,
                                    true,
                                ),
                        );
                        Ty::Unit
                    }
                };

                let var_id = self.new_var(final_ty.clone());
                self.scopes
                    .last_mut()
                    .expect("at least one scope should always be active")
                    .insert(
                        name.inner,
                        VarInfo {
                            id: var_id,
                            ty: final_ty,
                        },
                    );

                RStmt::Let {
                    var: var_id,
                    val: rval,
                }
            }
            StmtKind::Block(stmts) => {
                self.scopes.push(HashMap::new());
                let resolved = stmts.into_iter().map(|s| self.resolve_stmt(s)).collect();
                self.scopes.pop();
                RStmt::Block(resolved)
            }
            StmtKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let rcond = self.resolve_expr(cond);
                let rthen = Box::new(self.resolve_stmt(*then_branch));
                let relse = else_branch.map(|b| Box::new(self.resolve_stmt(*b)));
                RStmt::If {
                    cond: rcond,
                    then_branch: rthen,
                    else_branch: relse,
                }
            }
            StmtKind::While { cond, body } => {
                let rcond = self.resolve_expr(cond);
                let rbody = Box::new(self.resolve_stmt(*body));
                RStmt::While {
                    cond: rcond,
                    body: rbody,
                }
            }
            StmtKind::Expr(expr) => RStmt::Expr(self.resolve_expr(expr)),
            StmtKind::Asm(items) => {
                let resolved = items
                    .into_iter()
                    .map(|i| self.resolve_asm_item(i))
                    .collect();
                RStmt::Asm(resolved)
            }
            StmtKind::Return(expr) => {
                let ret = expr.map(|e| self.resolve_expr(e));
                RStmt::Return(ret)
            }
            StmtKind::Error => RStmt::Error,
        }
    }

    fn resolve_asm_item(&mut self, item: Spanned<AsmItem>) -> RAsmItem {
        match item.inner {
            AsmItem::In { reg, expr } => {
                let rexpr = self.resolve_expr(expr);
                RAsmItem::In {
                    reg: conv_reg(reg.inner),
                    expr: rexpr,
                }
            }
            AsmItem::Out { reg, lval } => {
                let (rlval, _ty) = self.resolve_lval(lval, item.span);
                RAsmItem::Out {
                    reg: conv_reg(reg.inner),
                    lval: rlval,
                }
            }
            AsmItem::Instr { mnemonic, args } => {
                let rargs = args
                    .into_iter()
                    .map(|a| match a.inner {
                        AsmOperand::Reg(r) => RAsmOperand::Reg(conv_reg(r)),
                        AsmOperand::Imm(n) => RAsmOperand::Imm(n),
                        AsmOperand::Error => RAsmOperand::Error,
                    })
                    .collect();
                RAsmItem::Instr {
                    mnemonic: mnemonic.inner,
                    args: rargs,
                }
            }
            AsmItem::Error => RAsmItem::Error,
        }
    }
}

fn conv_reg(r: Register) -> RRegister {
    match r {
        Register::R0 => RRegister::R0,
        Register::R1 => RRegister::R1,
        Register::R2 => RRegister::R2,
        Register::R3 => RRegister::R3,
        Register::R4 => RRegister::R4,
        Register::R5 => RRegister::R5,
        Register::R6 => RRegister::R6,
        Register::R7 => RRegister::R7,
    }
}

fn err_rexpr() -> RExpr {
    RExpr {
        kind: RExprKind::Error,
        ty: Ty::Unit,
    }
}
