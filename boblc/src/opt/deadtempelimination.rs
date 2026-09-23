use std::collections::HashSet;

use crate::tac::{TacInst, TacProgram, Temp};

pub fn optimize(program: &mut TacProgram) -> bool {
    let mut changed = false;
    for func in &mut program.funcs {
        let (c, new_body) = optimize_body(std::mem::take(&mut func.body));
        changed |= c;
        func.body = new_body;
    }
    changed
}

fn optimize_body(insts: Vec<TacInst>) -> (bool, Vec<TacInst>) {
    let mut used: HashSet<Temp> = HashSet::new();
    for inst in &insts {
        collect_used_temps(inst, &mut used);
    }

    let mut changed = false;
    let mut kept = Vec::with_capacity(insts.len());
    for inst in insts {
        match inst {
            TacInst::Dummy => changed = true,
            TacInst::Call {
                dst: Some(dst),
                callee,
            } => {
                changed |= !used.contains(&dst);
                kept.push(TacInst::Call {
                    dst: used.contains(&dst).then_some(dst),
                    callee,
                });
            }
            TacInst::CallDirect {
                dst: Some(dst),
                callee,
            } => {
                changed |= !used.contains(&dst);
                kept.push(TacInst::CallDirect {
                    dst: used.contains(&dst).then_some(dst),
                    callee,
                });
            }
            inst if let Some(dst) = pure_dst(&inst)
                && !used.contains(&dst) =>
            {
                changed = true;
            }
            inst => kept.push(inst),
        }
    }

    (changed, kept)
}

fn pure_dst(inst: &TacInst) -> Option<Temp> {
    match inst {
        TacInst::ConstInt { dst, .. }
        | TacInst::ConstStr { dst, .. }
        | TacInst::Copy { dst, .. }
        | TacInst::LoadVar { dst, .. }
        | TacInst::Load { dst, .. }
        | TacInst::AddrOf { dst, .. }
        | TacInst::LoadFunc { dst, .. }
        | TacInst::BinOp { dst, .. }
        | TacInst::UnOp { dst, .. }
        | TacInst::AsmOut { dst, .. } => Some(*dst),
        _ => None,
    }
}

fn collect_used_temps(inst: &TacInst, used: &mut HashSet<Temp>) {
    match inst {
        TacInst::ConstInt { .. } | TacInst::ConstStr { .. } => {}
        TacInst::Copy { src, .. } => {
            used.insert(*src);
        }
        TacInst::LoadVar { .. } => {}
        TacInst::StoreVar { src, .. } => {
            used.insert(*src);
        }
        TacInst::Load { addr, .. } => {
            used.insert(*addr);
        }
        TacInst::Store { addr, src } => {
            used.insert(*addr);
            used.insert(*src);
        }
        TacInst::AddrOf { .. } => {}
        TacInst::LoadFunc { .. } => {}
        TacInst::BinOp { lhs, rhs, .. } => {
            used.insert(*lhs);
            used.insert(*rhs);
        }
        TacInst::UnOp { src, .. } => {
            used.insert(*src);
        }
        TacInst::Param(t) => {
            used.insert(*t);
        }
        TacInst::ParamInt(_) | TacInst::ParamStr(_) => {}
        TacInst::Call { callee, .. } => {
            used.insert(*callee);
        }
        TacInst::CallDirect { .. } => {}
        TacInst::Label(_) | TacInst::Jump(_) => {}
        TacInst::JumpIfZero { cond, .. } => {
            used.insert(*cond);
        }
        TacInst::Return(val) => {
            if let Some(t) = val {
                used.insert(*t);
            }
        }
        TacInst::AsmIn { src, .. } => {
            used.insert(*src);
        }
        TacInst::AsmOut { .. } => {}
        TacInst::AsmRaw { .. } => {}
        TacInst::Dummy => {}
    }
}
