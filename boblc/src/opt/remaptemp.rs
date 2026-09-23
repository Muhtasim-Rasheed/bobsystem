use std::collections::HashMap;

use crate::tac::{TacInst, TacProgram, Temp};

pub fn optimize(program: &mut TacProgram) -> bool {
    let mut changed = false;
    for func in &mut program.funcs {
        let (c, num) = optimize_body(&mut func.body);
        func.num_temps = num;
        changed |= c;
    }
    changed
}

fn optimize_body(body: &mut [TacInst]) -> (bool, u32) {
    let mut remap = HashMap::<Temp, Temp>::new();
    let mut next_temp = 0u32;
    let mut changed = false;

    for inst in body.iter_mut() {
        let (c, new_inst) = remap_instruction(
            std::mem::replace(inst, TacInst::Dummy),
            &mut remap,
            &mut next_temp,
        );
        changed |= c;
        *inst = new_inst;
    }

    (changed, next_temp)
}

fn remap_temp(temp: Temp, remap: &mut HashMap<Temp, Temp>, next: &mut u32) -> Temp {
    *remap.entry(temp).or_insert_with(|| {
        let result = Temp(*next);
        *next += 1;
        result
    })
}

fn remap_instruction(
    inst: TacInst,
    remap: &mut HashMap<Temp, Temp>,
    next: &mut u32,
) -> (bool, TacInst) {
    macro_rules! x {
        ($temp:expr) => {
            remap_temp($temp, remap, next)
        };
    }
    let new_inst = match inst.clone() {
        TacInst::ConstInt { dst, val } => TacInst::ConstInt { dst: x!(dst), val },
        TacInst::ConstStr { dst, val } => TacInst::ConstStr { dst: x!(dst), val },
        TacInst::Copy { dst, src } => TacInst::Copy {
            dst: x!(dst),
            src: x!(src),
        },
        TacInst::LoadVar { dst, var } => TacInst::LoadVar { dst: x!(dst), var },
        TacInst::StoreVar { var, src } => TacInst::StoreVar { var, src: x!(src) },
        TacInst::Load { dst, addr } => TacInst::Load {
            dst: x!(dst),
            addr: x!(addr),
        },
        TacInst::Store { addr, src } => TacInst::Store {
            addr: x!(addr),
            src: x!(src),
        },
        TacInst::AddrOf { dst, var } => TacInst::AddrOf { dst: x!(dst), var },
        TacInst::LoadFunc { dst, func } => TacInst::LoadFunc { dst: x!(dst), func },
        TacInst::BinOp { dst, op, lhs, rhs } => TacInst::BinOp {
            dst: x!(dst),
            op,
            lhs: x!(lhs),
            rhs: x!(rhs),
        },
        TacInst::UnOp { dst, op, src } => TacInst::UnOp {
            dst: x!(dst),
            op,
            src: x!(src),
        },
        TacInst::Param(t) => TacInst::Param(x!(t)),
        TacInst::Call { dst, callee } => TacInst::Call {
            dst: dst.map(|v| x!(v)),
            callee: x!(callee),
        },
        TacInst::CallDirect { dst, callee } => TacInst::CallDirect {
            dst: dst.map(|v| x!(v)),
            callee,
        },
        TacInst::JumpIfZero { cond, target } => TacInst::JumpIfZero {
            cond: x!(cond),
            target,
        },
        TacInst::Return(Some(t)) => TacInst::Return(Some(x!(t))),
        TacInst::AsmIn { dst, src } => TacInst::AsmIn { dst, src: x!(src) },
        TacInst::AsmOut { dst, src } => TacInst::AsmOut { dst: x!(dst), src },
        other => other,
    };
    (inst != new_inst, new_inst)
}
