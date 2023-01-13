use std::fmt::Write;

use awint_ext::ExtAwi;
use Op::*;

use crate::{
    lowering::{OpDag, PNode},
    EvalError, EvalResult, Op,
};

impl OpDag {
    /// Assumes the node itself is evaluatable and all sources for `node` are
    /// literals. Note: decrements dependents but does not remove dead nodes.
    pub fn eval_node(&mut self, node: PNode) -> Result<(), EvalError> {
        let self_w = self[node].nzbw;
        let lit_op: Op<ExtAwi> =
            Op::translate(&self[node].op, |lhs: &mut [ExtAwi], rhs: &[PNode]| {
                for (lhs, rhs) in lhs.iter_mut().zip(rhs.iter()) {
                    if let Op::Literal(ref lit) = self[rhs].op {
                        *lhs = lit.clone();
                    } else {
                        unreachable!()
                    }
                }
            });
        match lit_op.eval(self_w) {
            EvalResult::Valid(x) | EvalResult::Pass(x) => {
                let len = self[node].op.operands_len();
                for i in 0..len {
                    let source = self[node].op.operands()[i];
                    self.dec_rc(source).unwrap();
                }
                self[node].op = Literal(x);
                Ok(())
            }
            EvalResult::Noop => {
                let operands = self[node].op.operands();
                let mut s = String::new();
                for op in operands {
                    write!(s, "{:?}, ", self[op]).unwrap();
                }
                Err(EvalError::OtherString(format!(
                    "`EvalResult::Noop` failure on operation node {} {:?} (\n{}\n)",
                    node, self[node].op, s
                )))
            }
            EvalResult::Error(e) => {
                if matches!(e, EvalError::Unevaluatable) {
                    Err(e)
                } else {
                    let operands = self[node].op.operands();
                    let mut s = String::new();
                    for op in operands {
                        write!(s, "{:?}, ", self[op]).unwrap();
                    }
                    Err(EvalError::OtherString(format!(
                        "`EvalResult::Error` failure (\n{:?}\n) on operation node {} {:?} (\n{}\n)",
                        e, node, self[node].op, s
                    )))
                }
            }
        }
    }

    /// Evaluates the source tree of `leaf` as much as possible. Only evaluates
    /// nodes less than `visit`, evaluated nodes have their visit number set
    /// to `visit`.
    pub fn eval_tree(&mut self, leaf: PNode, visit: u64) -> Result<(), EvalError> {
        if self.a[leaf].visit >= visit {
            return Ok(())
        }
        // DFS from leaf to roots
        // the bool is set to false when an unevaluatabe node is in the sources
        let mut path: Vec<(usize, PNode, bool)> = vec![(0, leaf, true)];
        loop {
            let (i, p, all_literals) = path[path.len() - 1];
            let ops = self[p].op.operands();
            if ops.is_empty() {
                // reached a root
                path.pop().unwrap();
                if path.is_empty() {
                    break
                }
                path.last_mut().unwrap().0 += 1;
                path.last_mut().unwrap().2 &= self[p].op.is_literal();
            } else if i >= ops.len() {
                // checked all sources
                path.pop().unwrap();
                if all_literals {
                    match self.eval_node(p) {
                        Ok(()) => {}
                        Err(EvalError::Unevaluatable) => {}
                        Err(e) => {
                            self[p].err = Some(e.clone());
                            return Err(e)
                        }
                    }
                }
                if path.is_empty() {
                    break
                }
                path.last_mut().unwrap().2 &= all_literals;
            } else {
                let p_next = ops[i];
                if self[p_next].visit >= visit {
                    // peek at node for evaluatableness but do not visit node, this prevents
                    // exponential growth
                    path.last_mut().unwrap().0 += 1;
                    path.last_mut().unwrap().2 &= self[p_next].op.is_literal();
                } else {
                    self[p_next].visit = visit;
                    path.push((0, p_next, true));
                }
            }
        }
        Ok(())
    }

    /// Evaluates all nodes
    pub fn eval_all(&mut self) -> Result<(), EvalError> {
        self.visit_gen += 1;
        let (mut p, mut b) = self.a.first_ptr();
        loop {
            if b {
                break
            }
            self.eval_tree(p, self.visit_gen)?;
            self.a.next_ptr(&mut p, &mut b);
        }
        Ok(())
    }
}
