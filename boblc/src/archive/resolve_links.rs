use crate::parse::{FunctionDecl, Program, Word};

enum ControlFlowWord {
    If,
    While,
    Do,
}

pub fn resolve_links(ast: &mut Program) -> Result<(), (String, usize)> {
    for function in &mut ast.functions {
        let mut stack = vec![];

        if let FunctionDecl::Defined { name, body } = function {
            for i in 0..body.len() {
                match body[i] {
                    Word::If(_) => {
                        stack.push((ControlFlowWord::If, i));
                    }
                    Word::While => {
                        stack.push((ControlFlowWord::While, i));
                    }
                    Word::Do(_) => {
                        stack.push((ControlFlowWord::Do, i));
                    }
                    Word::Else(_) => {
                        let Some((ControlFlowWord::If, if_i)) = stack.pop() else {
                            return Err((name.clone(), i));
                        };

                        body[if_i] = Word::If(i + 1);
                        body[i] = Word::Else(0);

                        stack.push((ControlFlowWord::If, i));
                    }
                    Word::End(_) => {
                        let Some((c, j)) = stack.pop() else {
                            return Err((name.clone(), i));
                        };

                        match c {
                            ControlFlowWord::If => match body[j] {
                                Word::If(_) => {
                                    body[j] = Word::If(i + 1);
                                    body[i] = Word::End(None);
                                }
                                Word::Else(_) => {
                                    body[j] = Word::Else(i + 1);
                                    body[i] = Word::End(None);
                                }

                                _ => unreachable!(),
                            },
                            ControlFlowWord::While => {
                                return Err((name.clone(), i));
                            }
                            ControlFlowWord::Do => {
                                let Some((ControlFlowWord::While, k)) = stack.pop() else {
                                    return Err((name.clone(), i));
                                };

                                body[j] = Word::Do(i + 1);
                                body[i] = Word::End(Some(k));
                            }
                        }
                    }

                    _ => {}
                }
            }

            if !stack.is_empty() {
                return Err((name.clone(), body.len()));
            }
        }
    }

    Ok(())
}
