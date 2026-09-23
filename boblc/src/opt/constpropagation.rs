use std::collections::{HashMap, HashSet};

use crate::{
    rast::FuncId,
    tac::{TacInst, TacProgram, Temp},
};

enum Constant {
    Int(u16),
    Str(String),
    Func(FuncId),
}

pub fn optimize(program: &mut TacProgram) -> bool {
    let mut changed = false;
    for func in &mut program.funcs {
        let (new_body, c) = optimize_body(std::mem::take(&mut func.body));
        func.body = new_body;
        changed |= c;
    }
    changed
}

fn optimize_body(body: Vec<TacInst>) -> (Vec<TacInst>, bool) {
    let mut known_consts: HashMap<Temp, (Constant, usize)> = HashMap::new();
    let mut remove = HashSet::new();
    let mut output = Vec::with_capacity(body.len());
    let mut changed = false;

    for inst in body {
        match inst {
            TacInst::ConstInt { dst, val } => {
                let output_index = output.len();
                known_consts.insert(dst, (Constant::Int(val), output_index));
                output.push(TacInst::ConstInt { dst, val });
            }
            TacInst::ConstStr { dst, val } => {
                let output_index = output.len();
                known_consts.insert(dst, (Constant::Str(val.clone()), output_index));
                output.push(TacInst::ConstStr { dst, val });
            }
            TacInst::LoadFunc { dst, func } => {
                let output_index = output.len();
                known_consts.insert(dst, (Constant::Func(func), output_index));
                output.push(TacInst::LoadFunc { dst, func });
            }
            TacInst::Call { dst, callee } => {
                if let Some(&(Constant::Func(func), load_index)) = known_consts.get(&callee) {
                    output.push(TacInst::CallDirect { dst, callee: func });
                    remove.insert(load_index);
                    changed = true;
                } else {
                    output.push(TacInst::Call { dst, callee });
                }

                if let Some(dst) = dst {
                    known_consts.remove(&dst);
                }
            }
            TacInst::Param(temp) => {
                if let Some((constant, load_index)) = known_consts.get(&temp) {
                    match constant {
                        Constant::Int(value) => {
                            output.push(TacInst::ParamInt(value.clone()));
                            remove.insert(*load_index);
                            changed = true;
                        }
                        Constant::Str(value) => {
                            output.push(TacInst::ParamStr(value.clone()));
                            remove.insert(*load_index);
                            changed = true;
                        }
                        Constant::Func(_) => {
                            output.push(TacInst::Param(temp));
                        }
                    }
                } else {
                    output.push(TacInst::Param(temp));
                }
            }
            other => {
                for dst in assigned_temps(&other) {
                    known_consts.remove(&dst);
                }

                output.push(other);
            }
        }
    }

    let output = output
        .into_iter()
        .enumerate()
        .filter_map(|(i, inst)| (!remove.contains(&i)).then_some(inst))
        .collect();

    (output, changed)
}

fn assigned_temps(inst: &TacInst) -> Vec<Temp> {
    match inst {
        TacInst::ConstInt { dst, .. } // redundant
        | TacInst::ConstStr { dst, .. } // redundant
        | TacInst::Copy { dst, .. }
        | TacInst::LoadVar { dst, .. }
        | TacInst::Load { dst, .. }
        | TacInst::AddrOf { dst, .. }
        | TacInst::LoadFunc { dst, .. } // redundant
        | TacInst::BinOp { dst, .. }
        | TacInst::UnOp { dst, .. }
        | TacInst::Call { dst: Some(dst), .. } // redundant
        | TacInst::CallDirect { dst: Some(dst), .. } // redundant
        | TacInst::AsmOut { dst, .. } => vec![*dst],
        _ => vec![],
    }
}
