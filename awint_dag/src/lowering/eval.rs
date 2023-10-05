use std::fmt::Write;

use awint_ext::Awi;
use awint_macro_internals::triple_arena::Advancer;
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
        let lit_op: Op<Awi> = Op::translate(&self[node].op, |lhs: &mut [Awi], rhs: &[PNode]| {
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
                let mut p_next = ops[i];
                if self[p_next].visit >= visit {
                    // peek at node for evaluatableness but do not visit node, this prevents
                    // exponential growth
                    path.last_mut().unwrap().0 += 1;
                    path.last_mut().unwrap().2 &= self[p_next].op.is_literal();
                    while let Op::Copy([a]) = self[p_next].op {
                        // special optimization case: forward Copies
                        self[p].op.operands_mut()[i] = a;
                        self.dec_rc(p_next).unwrap();
                        p_next = a;
                    }
                } else {
                    self[p_next].visit = visit;
                    path.push((0, p_next, true));
                }
            }
        }
        Ok(())
    }

    /// Evaluates all nodes. This also checks assertions, but not as strictly as
    /// `assert_assertions` which makes sure no assertions are unevaluated.
    pub fn eval_all(&mut self) -> Result<(), EvalError> {
        self.visit_gen += 1;
        let mut adv = self.a.advancer();
        while let Some(p) = adv.advance(&self.a) {
            self.eval_tree(p, self.visit_gen)?;
        }
        self.assert_assertions_weak()?;
        self.unnote_true_assertions();
        Ok(())
    }

    /// Unnotes assertions that have been evaluated to be true, done by
    /// `eval_all` and `lower_all`
    pub fn unnote_true_assertions(&mut self) {
        let mut i = 0;
        loop {
            if i >= self.assertions.len() {
                break
            }
            let p_note = self.assertions[i];
            let p_node = self.note_arena[p_note];
            let mut removed = false;
            if let Op::Literal(ref lit) = self[p_node].op {
                if !lit.is_zero() {
                    self.unnote_pnote(p_note);
                    self.assertions.swap_remove(i);
                    removed = true;
                }
            }
            if !removed {
                i += 1;
            }
        }
    }

    /// Deletes all nodes unused by any noted node.
    pub fn delete_unused_nodes(&mut self) {
        let mut adv = self.a.advancer();
        while let Some(p) = adv.advance(&self.a) {
            if self[p].rc == 0 {
                self.trim_zero_rc_tree(p).unwrap();
            }
        }
    }
}
