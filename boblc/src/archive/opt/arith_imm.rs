use crate::parse::{FunctionDecl, Program, Word};

#[derive(Clone)]
enum StackVal {
    Int(i64),
    Str(String),
    Unknown,
}

pub fn optimize(program: &mut Program) -> bool {
    use StackVal::*;
    let mut changed = false;
    for function in &mut program.functions {
        if let FunctionDecl::Defined { name: _, body } = function {
            let mut new_body = Vec::new();
            let mut stack = vec![];
            let mut return_stack = vec![];
            for (i, word) in std::mem::take(body).into_iter().enumerate() {
                let mut same_word = true;
                new_body.push(None);
                match &word {
                    Word::PushInt(int) => stack.push((Int(*int), i)),
                    Word::PushString(s) => stack.push((Str(s.clone()), i)),
                    Word::Drop => {
                        stack.pop();
                    }
                    Word::Add => match stack.pop() {
                        Some((Int(int), j)) => {
                            same_word = false;
                            new_body[i] = Some(Word::AddInt(int));
                            new_body[j] = None;
                        }
                        Some((Str(s), j)) => {
                            same_word = false;
                            new_body[i] = Some(Word::AddStr(s));
                            new_body[j] = None;
                        }
                        Some((Unknown, j)) => {
                            stack.push((Unknown, j));
                        }
                        None => {}
                    },
                    Word::And => match stack.pop() {
                        Some((Int(int), j)) => {
                            same_word = false;
                            new_body[i] = Some(Word::AndInt(int));
                            new_body[j] = None;
                        }
                        Some((Str(s), j)) => {
                            same_word = false;
                            new_body[i] = Some(Word::AndStr(s));
                            new_body[j] = None;
                        }
                        Some((Unknown, j)) => {
                            stack.push((Unknown, j));
                        }
                        None => {}
                    },
                    Word::Not => match stack.pop() {
                        Some((Int(int), j)) => {
                            same_word = false;
                            new_body[i] = Some(Word::NotInt(int));
                            new_body[j] = None;
                        }
                        Some((Str(s), j)) => {
                            same_word = false;
                            new_body[i] = Some(Word::NotStr(s));
                            new_body[j] = None;
                        }
                        Some((Unknown, j)) => {
                            stack.push((Unknown, j));
                        }
                        None => {}
                    },
                    Word::Dup => {
                        if let Some(a) = stack.last().cloned() {
                            stack.push(a);
                        }
                    }
                    Word::TwoDup => {
                        if stack.len() >= 2 {
                            let a = stack[stack.len() - 2].clone();
                            let b = stack[stack.len() - 1].clone();

                            stack.push(a);
                            stack.push(b);
                        } else {
                            stack.clear();
                        }
                    }
                    Word::Swap => {
                        if stack.len() >= 2 {
                            let len = stack.len();
                            stack.swap(len - 1, len - 2);
                        } else {
                            stack.clear();
                        }
                    }
                    Word::Rot => {
                        if stack.len() >= 3 {
                            let len = stack.len();

                            // a b c -> b c a
                            let a = stack.remove(len - 3);
                            stack.push(a);
                        } else {
                            stack.clear();
                        }
                    }
                    Word::If(_) | Word::Do(_) => {
                        stack.pop();
                        stack.clear();
                    }
                    Word::Else(_) => {
                        stack.clear();
                    }
                    Word::While => {
                        stack.clear();
                    }
                    Word::End(_) => {
                        stack.clear();
                    }
                    Word::Dr => {
                        if let Some(value) = stack.pop() {
                            return_stack.push(value);
                        } else {
                            return_stack.push((Unknown, i));
                        }
                    }
                    Word::Rd => {
                        if let Some(value) = return_stack.pop() {
                            stack.push(value);
                        } else {
                            stack.push((Unknown, i));
                        }
                    }
                    Word::Trap1 | Word::Trap2 => {
                        // dont assume anything about the stack across traps
                        stack.clear();
                    }
                    Word::Call(_) => {
                        // a function can consume/produce arbitrary stack values
                        stack.clear();
                    }
                    Word::AddInt(_)
                    | Word::AddStr(_)
                    | Word::AndInt(_)
                    | Word::AndStr(_)
                    | Word::NotInt(_)
                    | Word::NotStr(_) => {
                        stack.clear();
                    }
                }
                if same_word {
                    new_body[i] = Some(word);
                } else {
                    changed = true;
                }
            }
            for inst in new_body.into_iter().filter_map(|v| v) {
                body.push(inst);
            }
        }
    }
    changed
}
