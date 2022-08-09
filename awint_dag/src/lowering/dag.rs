//! This DAG is for lowering into a LUT-only DAG

use std::{
    borrow::Borrow,
    num::NonZeroUsize,
    ops::{Index, IndexMut},
    vec,
};

use awint_core::Bits;
use awint_internals::BITS;
use triple_arena::Arena;
use Op::*;

use crate::{
    common::{
        state::{get_state, next_state_visit_gen, set_state_node_map, PState},
        EvalError, Op, PNode,
    },
    lowering::Node,
};

#[derive(Debug)]
pub struct Dag {
    pub dag: Arena<PNode, Node<PNode>>,
    /// for keeping nodes alive and having an ordered list for identification
    pub noted: Vec<Option<PNode>>,
    /// A kind of generation counter tracking the highest `visit_num` number
    pub visit_gen: u64,
    /// for capacity reuse in `add_group`
    pub tmp_stack: Vec<(usize, PNode, PState)>,
    /// for capacity reuse in `dec_rc`
    pub tmp_pnode_stack: Vec<PNode>,
}

impl<B: Borrow<PNode>> Index<B> for Dag {
    type Output = Node<PNode>;

    fn index(&self, index: B) -> &Node<PNode> {
        self.dag.get(*index.borrow()).unwrap()
    }
}

impl<B: Borrow<PNode>> IndexMut<B> for Dag {
    fn index_mut(&mut self, index: B) -> &mut Node<PNode> {
        self.dag.get_mut(*index.borrow()).unwrap()
    }
}

impl Dag {
    /// Constructs a directed acyclic graph from the leaf sinks of a mimicking
    /// version. The optional `note`s should be included in the DAG reachable
    /// from the `leaves`, and should be `Opaque` if they should remain
    /// unmutated through optimizations.
    ///
    /// If an error occurs, the DAG (which may be in an unfinished or completely
    /// broken state) is still returned along with the error enum, so that debug
    /// tools like `render_to_svg_file` can be used.
    pub fn new(leaves: &[PState], noted: &[PState]) -> (Self, Result<(), EvalError>) {
        let mut res = Self {
            dag: Arena::new(),
            noted: vec![],
            visit_gen: 0,
            tmp_stack: vec![],
            tmp_pnode_stack: vec![],
        };
        let err = res.add_group(leaves, noted, true, 0, None);
        (res, err)
    }

    /// Adds the `leaves` and their source trees to `self`. For each `noted`
    /// node in order, the translated node is pushed to `self.noted` or `None`
    /// is pushed if it is not found in the source trees. If `inc_noted_rc` and
    /// a `noted` node is found, the rc is incremented. The visit numbers of
    /// all the added nodes are set to `self.visit_gen`. Note: the leaves
    /// and all their preceding nodes should not share with previously
    /// existing nodes in this DAG or else there will be duplication.
    pub fn add_group(
        &mut self,
        leaves: &[PState],
        noted: &[PState],
        inc_noted_rc: bool,
        visit: u64,
        mut added: Option<&mut Vec<PNode>>,
    ) -> Result<(), EvalError> {
        // this is for the state side visits not the `visit` for `Node`s
        let state_visit = next_state_visit_gen();
        self.tmp_stack.clear();
        for leaf in leaves {
            let leaf_state = get_state(*leaf).unwrap();
            if leaf_state.visit != state_visit {
                let p_leaf = self.dag.insert_with(|p_this| Node {
                    nzbw: get_state(*leaf).unwrap().nzbw,
                    visit,
                    p_this,
                    op: Op::Invalid,
                    rc: 0,
                    err: None,
                });
                set_state_node_map(*leaf, state_visit, p_leaf);
                if let Some(ref mut v) = added {
                    v.push(p_leaf);
                }
                self.tmp_stack.push((0, p_leaf, *leaf));
                loop {
                    let (current_i, current_p_node, current_p_state) =
                        *self.tmp_stack.last().unwrap();
                    let state = get_state(current_p_state).unwrap();
                    if let Some(t) = Op::translate_root(&state.op) {
                        // reached a root
                        self[current_p_node].op = t;
                        self[current_p_node].nzbw = state.nzbw;
                        self.tmp_stack.pop().unwrap();
                        if let Some((i, ..)) = self.tmp_stack.last_mut() {
                            *i += 1;
                        } else {
                            break
                        }
                    } else if current_i >= state.op.num_operands() {
                        // all operands should be ready
                        self[current_p_node].op =
                            Op::translate(&state.op, |lhs: &mut [PNode], rhs: &[PState]| {
                                for (lhs, rhs) in lhs.iter_mut().zip(rhs.iter()) {
                                    *lhs = get_state(*rhs).unwrap().node_map;
                                }
                            });
                        self[current_p_node].nzbw = state.nzbw;
                        self.tmp_stack.pop().unwrap();
                        if let Some((i, ..)) = self.tmp_stack.last_mut() {
                            *i += 1;
                        } else {
                            break
                        }
                    } else {
                        // check next operand
                        let p_next = state.op.operands()[current_i];
                        let next_state = get_state(p_next).unwrap();
                        if next_state.visit == state_visit {
                            // already explored
                            self[next_state.node_map].rc += 1;
                            if let Some((i, ..)) = self.tmp_stack.last_mut() {
                                *i += 1;
                            } else {
                                break
                            }
                        } else {
                            let p = self.dag.insert_with(|p_this| Node {
                                rc: 1,
                                p_this,
                                nzbw: state.nzbw,
                                op: Op::Invalid,
                                err: None,
                                visit,
                            });
                            set_state_node_map(p_next, state_visit, p);
                            if let Some(ref mut v) = added {
                                v.push(p);
                            }
                            self.tmp_stack.push((0, p, state.op.operands()[current_i]));
                        }
                    }
                }
            }
        }
        // handle the noted
        for p_noted in noted {
            let state = get_state(*p_noted).unwrap();
            if state.visit == state_visit {
                let p_node = state.node_map;
                if inc_noted_rc {
                    self[p_node].rc += 1;
                }
                self.noted.push(Some(p_node));
            } else {
                self.noted.push(None);
            }
        }
        Ok(())
    }

    pub fn verify_integrity(&mut self) -> Result<(), EvalError> {
        for v in self.dag.vals() {
            if let Some(ref err) = v.err {
                return Err(err.clone())
            }
        }
        for p in self.noted.iter().flatten() {
            if self.dag.get(*p).is_none() {
                return Err(EvalError::InvalidPtr)
            }
        }
        // TODO there should be more checks
        Ok(())
    }

    /// Returns a list of pointers to all nodes in no particular order
    pub fn ptrs(&self) -> Vec<PNode> {
        self.dag.ptrs().collect()
    }

    /// Assumes `ptr` is a literal
    pub fn lit(&self, ptr: PNode) -> &Bits {
        if let Literal(ref lit) = self[ptr].op {
            lit
        } else {
            panic!("node was not a literal")
        }
    }

    /// Assumes `ptr` is a literal. Returns `None` if the literal does not have
    /// bitwidth 1.
    pub fn bool(&self, ptr: PNode) -> Result<bool, EvalError> {
        if let Literal(ref lit) = self[ptr].op {
            if lit.bw() == 1 {
                Ok(lit.to_bool())
            } else {
                Err(EvalError::WrongBitwidth)
            }
        } else {
            panic!("node was not a literal")
        }
    }

    /// Assumes `ptr` is a literal. Returns `None` if the literal does not have
    /// bitwidth `usize::BITS`.
    pub fn usize(&self, ptr: PNode) -> Result<usize, EvalError> {
        if let Literal(ref lit) = self[ptr].op {
            if lit.bw() == BITS {
                Ok(lit.to_usize())
            } else {
                Err(EvalError::WrongBitwidth)
            }
        } else {
            panic!("node was not a literal")
        }
    }

    pub fn get_bw<B: Borrow<PNode>>(&self, ptr: B) -> NonZeroUsize {
        self[ptr].nzbw
    }

    /// Decrements the reference count on `p`, removing its tree if the count
    /// goes to 0.
    pub fn dec_rc(&mut self, p: PNode) -> Result<(), EvalError> {
        self[p].rc = if let Some(x) = self[p].rc.checked_sub(1) {
            x
        } else {
            return Err(EvalError::OtherStr("tried to subtract a 0 reference count"))
        };
        if self[p].rc == 0 {
            self.tmp_pnode_stack.clear();
            self.tmp_pnode_stack.push(p);
            while let Some(p) = self.tmp_pnode_stack.pop() {
                let mut delete = false;
                if let Some(node) = self.dag.get(p) {
                    if node.rc == 0 {
                        delete = true;
                    }
                }
                if delete {
                    for i in 0..self[p].op.num_operands() {
                        let op = self[p].op.operands()[i];
                        self[op].rc = if let Some(x) = self[op].rc.checked_sub(1) {
                            x
                        } else {
                            return Err(EvalError::OtherStr("tried to subtract a 0 reference count"))
                        };
                        self.tmp_pnode_stack.push(op);
                    }
                    self.dag.remove(p).unwrap();
                }
            }
        }
        Ok(())
    }

    /// Forbidden meta pseudo-DSL techniques in which the node at `ptr` is
    /// replaced by a set of lowered `PState` nodes with interfaces `output`
    /// and `operands` corresponding to the original interfaces. Newly added
    /// nodes not including `ptr` are colored with `visit`.
    pub fn graft(
        &mut self,
        ptr: PNode,
        visit: u64,
        output_and_operands: &[PState],
    ) -> Result<(), EvalError> {
        #[cfg(debug_assertions)]
        {
            if (self[ptr].op.num_operands() + 1) != output_and_operands.len() {
                return Err(EvalError::WrongNumberOfOperands)
            }
            for (i, op) in self[ptr].op.operands().iter().enumerate() {
                let current_state = get_state(output_and_operands[i + 1]).unwrap();
                if self[op].nzbw != current_state.nzbw {
                    return Err(EvalError::OtherString(format!(
                        "operand {}: a bitwidth of {:?} is trying to be grafted to a bitwidth of \
                         {:?}",
                        i, current_state.nzbw, self[op].nzbw
                    )))
                }
                if !current_state.op.is_opaque() {
                    return Err(EvalError::ExpectedOpaque)
                }
            }
            if self[ptr].nzbw != get_state(output_and_operands[0]).unwrap().nzbw {
                return Err(EvalError::WrongBitwidth)
            }
        }
        // note we do not increment the rc because we immediately remove the node's
        // special status
        let err = self.add_group(
            &[output_and_operands[0]],
            output_and_operands,
            false,
            visit,
            None,
        );
        //self.render_to_svg_file(std::path::PathBuf::from("debug.svg"))
        //    .unwrap();
        err?;
        //self.verify_integrity()?;
        let start = self.noted.len() - output_and_operands.len();

        // graft inputs
        for i in 0..(output_and_operands.len() - 1) {
            let grafted = self.noted[start + i + 1];
            let graftee = self[ptr].op.operands()[i];
            if let Some(grafted) = grafted {
                // change the grafted `Opaque` to a `Copy` that routes to the graftee instead of
                // needing to change all the operands of potentially many internal nodes.
                self[grafted].op = Copy([graftee]);
            } else {
                // else the operand is not used because it was optimized away
                self.dec_rc(graftee).unwrap();
            }
        }

        // graft output
        // remove the temporary noted nodes
        let p = self.noted[start].unwrap();
        // this will replace the graftee's location to avoid changing downstream nodes
        let grafted = self.dag.remove(p).unwrap();
        let graftee = self.dag.replace_and_keep_gen(ptr, grafted).unwrap();

        // preserve original reference count
        self[ptr].rc = graftee.rc;
        self[ptr].p_this = graftee.p_this;

        // reset the `noted` to its original state
        self.noted.drain(start..);
        Ok(())
    }

    /// Always renders to file, and then returns errors
    #[cfg(feature = "debug")]
    pub fn render_to_svg_file(&mut self, out_file: std::path::PathBuf) -> Result<(), EvalError> {
        let res = self.verify_integrity();
        triple_arena_render::render_to_svg_file(&self.dag, false, out_file).unwrap();
        res
    }
}
