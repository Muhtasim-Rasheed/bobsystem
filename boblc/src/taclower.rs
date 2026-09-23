use crate::{
    opt,
    rast::{
        FuncId, RAsmItem, RAsmOperand, RExpr, RExprKind, RFunction, RLValue, RProgram, RRegister,
        RStmt, VarId,
    },
    tac::{Label, TacAsmOperand, TacFunction, TacInst, TacProgram, TacRegister, Temp},
    ty::Ty,
};

// all checking is done in the resolver, so we dont need the compiler ctx here

pub struct Lowerer {
    rprogram: RProgram,
    tacprogram: TacProgram,

    next_temp: u32,
    next_label: u32,
    current_function: Option<TacFunction>,
}

impl Lowerer {
    pub fn new(rprogram: RProgram) -> Self {
        Self {
            rprogram,
            tacprogram: TacProgram { funcs: vec![] },
            next_temp: 0,
            next_label: 0,
            current_function: None,
        }
    }

    fn new_func(&mut self, id: FuncId, locals: Vec<Ty>, params: Vec<VarId>) {
        self.finish_func();

        self.next_temp = 0;
        self.next_label = 0;
        self.current_function = Some(TacFunction {
            id,
            params,
            num_temps: 0,
            locals,
            body: vec![],
        });
    }

    fn finish_func(&mut self) {
        if let Some(func) = self.current_function.take() {
            self.tacprogram.funcs.push(func);
        }
    }

    fn new_temp(&mut self) -> Temp {
        let func = self
            .current_function
            .as_mut()
            .expect("the lowerer should be lowering a function");
        let temp = Temp(self.next_temp);
        func.num_temps += 1;
        self.next_temp += 1;
        temp
    }

    fn new_label(&mut self) -> Label {
        let label = Label(self.next_label);
        self.next_label += 1;
        label
    }

    fn emit(&mut self, inst: TacInst) {
        let func = self
            .current_function
            .as_mut()
            .expect("the lowerer should be lowering a function");
        func.body.push(inst);
    }

    fn lower_lvalue_addr(&mut self, lval: RLValue) -> Temp {
        match lval {
            RLValue::Var(var) => {
                let dst = self.new_temp();
                self.emit(TacInst::AddrOf { dst, var });
                dst
            }
            RLValue::Deref(expr) => self.lower_expr(*expr),
            RLValue::Error => unreachable!(),
        }
    }

    fn lower_expr(&mut self, expr: RExpr) -> Temp {
        match expr.kind {
            RExprKind::IntLit(i) => {
                let dst = self.new_temp();
                self.emit(TacInst::ConstInt { dst, val: i });
                dst
            }
            RExprKind::CharLit(c) => {
                let dst = self.new_temp();
                self.emit(TacInst::ConstInt { dst, val: c as u16 });
                dst
            }
            RExprKind::StrLit(s) => {
                let dst = self.new_temp();
                self.emit(TacInst::ConstStr { dst, val: s });
                dst
            }
            RExprKind::LValue(lval) => match lval {
                RLValue::Var(var) => {
                    let dst = self.new_temp();
                    self.emit(TacInst::LoadVar { dst, var });
                    dst
                }
                RLValue::Deref(expr) => {
                    let addr = self.lower_expr(*expr);
                    let dst = self.new_temp();
                    self.emit(TacInst::Load { dst, addr });
                    dst
                }
                RLValue::Error => unreachable!(),
            },
            RExprKind::Function(func) => {
                let dst = self.new_temp();
                self.emit(TacInst::LoadFunc { dst, func });
                dst
            }
            RExprKind::Ref(lval) => self.lower_lvalue_addr(lval),
            RExprKind::Binary { left, op, right } => {
                let lhs = self.lower_expr(*left);
                let rhs = self.lower_expr(*right);
                let dst = self.new_temp();
                self.emit(TacInst::BinOp { dst, op, lhs, rhs });
                dst
            }
            RExprKind::Unary { expr, op } => {
                let src = self.lower_expr(*expr);
                let dst = self.new_temp();
                self.emit(TacInst::UnOp { dst, op, src });
                dst
            }
            RExprKind::Assign { target, val } => {
                let val = self.lower_expr(*val);

                match target {
                    RLValue::Var(var) => {
                        self.emit(TacInst::StoreVar { var, src: val });
                    }
                    RLValue::Deref(expr) => {
                        let addr = self.lower_expr(*expr);
                        self.emit(TacInst::Store { addr, src: val });
                    }
                    RLValue::Error => unreachable!(),
                }

                val
            }
            RExprKind::Call { callee, args } => {
                let callee = self.lower_expr(*callee);
                let dst = self.new_temp();
                for arg in args {
                    let arg = self.lower_expr(arg);
                    self.emit(TacInst::Param(arg));
                }
                self.emit(TacInst::Call {
                    dst: Some(dst),
                    callee,
                });
                dst
            }
            RExprKind::Error => unreachable!(),
        }
    }

    fn lower_stmt(&mut self, stmt: RStmt) {
        match stmt {
            RStmt::Let { var, val } => {
                if let Some(val) = val {
                    let src = self.lower_expr(val);

                    self.emit(TacInst::StoreVar { var, src });
                }
            }
            RStmt::Block(stmts) => {
                for stmt in stmts {
                    self.lower_stmt(stmt);
                }
            }
            RStmt::Expr(expr) => {
                self.lower_expr(expr);
            }
            RStmt::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let else_label = self.new_label();
                let end_label = self.new_label();

                let cond = self.lower_expr(cond);

                self.emit(TacInst::JumpIfZero {
                    cond,
                    target: else_label,
                });

                self.lower_stmt(*then_branch);

                if let Some(else_branch) = else_branch {
                    self.emit(TacInst::Jump(end_label));

                    self.emit(TacInst::Label(else_label));
                    self.lower_stmt(*else_branch);

                    self.emit(TacInst::Label(end_label));
                } else {
                    self.emit(TacInst::Label(else_label));
                }
            }
            RStmt::While { cond, body } => {
                let cond_label = self.new_label();
                let exit_label = self.new_label();

                self.emit(TacInst::Label(cond_label));

                let cond = self.lower_expr(cond);
                self.emit(TacInst::JumpIfZero {
                    cond,
                    target: exit_label,
                });

                self.lower_stmt(*body);
                self.emit(TacInst::Jump(cond_label));

                self.emit(TacInst::Label(exit_label));
            }
            RStmt::Asm(items) => {
                for item in items {
                    match item {
                        RAsmItem::In { reg, expr } => {
                            let src = self.lower_expr(expr);
                            self.emit(TacInst::AsmIn {
                                dst: conv_reg(reg),
                                src,
                            });
                        }
                        RAsmItem::Out { reg, lval } => {
                            let dst = self.new_temp();
                            self.emit(TacInst::AsmOut {
                                dst,
                                src: conv_reg(reg),
                            });
                            match lval {
                                RLValue::Var(var) => {
                                    self.emit(TacInst::StoreVar { var, src: dst });
                                }
                                RLValue::Deref(expr) => {
                                    let addr = self.lower_expr(*expr);
                                    self.emit(TacInst::Store { addr, src: dst });
                                }
                                RLValue::Error => unreachable!(),
                            }
                        }
                        RAsmItem::Instr { mnemonic, args } => {
                            self.emit(TacInst::AsmRaw {
                                mnemonic,
                                args: args
                                    .into_iter()
                                    .map(|a| match a {
                                        RAsmOperand::Reg(reg) => TacAsmOperand::Reg(conv_reg(reg)),
                                        RAsmOperand::Imm(imm) => TacAsmOperand::Imm(imm),
                                        RAsmOperand::Error => unreachable!(),
                                    })
                                    .collect(),
                            });
                        }
                        RAsmItem::Error => unreachable!(),
                    }
                }
            }
            RStmt::Return(expr) => {
                let ret = expr.map(|e| self.lower_expr(e));
                self.emit(TacInst::Return(ret));
            }
            RStmt::Error => unreachable!(),
        }
    }

    fn lower_function(&mut self, func: RFunction) {
        self.new_func(
            func.id,
            func.locals,
            func.params.into_iter().map(|v| v.0).collect(),
        );
        self.lower_stmt(func.body);
        self.finish_func();
    }

    pub fn lower(mut self) -> TacProgram {
        for module in std::mem::take(&mut self.rprogram.modules) {
            for func in module.functions {
                self.lower_function(func);
            }
        }

        self.finish_func();

        let mut changed = true;
        while changed {
            changed = false;
            changed |= opt::constpropagation::optimize(&mut self.tacprogram);
            changed |= opt::deadtempelimination::optimize(&mut self.tacprogram);
            changed |= opt::remaptemp::optimize(&mut self.tacprogram);
        }

        self.into_tacprogram()
    }

    pub fn into_tacprogram(self) -> TacProgram {
        self.tacprogram
    }
}

fn conv_reg(r: RRegister) -> TacRegister {
    match r {
        RRegister::R0 => TacRegister::R0,
        RRegister::R1 => TacRegister::R1,
        RRegister::R2 => TacRegister::R2,
        RRegister::R3 => TacRegister::R3,
        RRegister::R4 => TacRegister::R4,
        RRegister::R5 => TacRegister::R5,
        RRegister::R6 => TacRegister::R6,
        RRegister::R7 => TacRegister::R7,
    }
}
