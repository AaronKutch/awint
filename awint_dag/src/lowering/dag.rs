//! This DAG is for lowering into a LUT-only DAG

use std::{
    borrow::Borrow,
    collections::{hash_map::Entry, HashMap},
    num::NonZeroUsize,
    ops::{Index, IndexMut},
    rc::Rc,
};

use awint_core::Bits;
use awint_internals::BITS;
use triple_arena::{Arena, Ptr, PtrTrait};
use Op::*;

use crate::{
    common::{EvalError, Op, State},
    lowering::{Node, PtrEqRc},
};

#[derive(Debug)]
pub struct Dag<P: PtrTrait> {
    pub dag: Arena<P, Node<P>>,
    /// for keeping nodes alive and having an ordered list for identification
    pub noted: Vec<Ptr<P>>,
    /// A kind of generation counter tracking the highest `visit_num` number
    pub visit_gen: u64,
}

impl<P: PtrTrait, B: Borrow<Ptr<P>>> Index<B> for Dag<P> {
    type Output = Node<P>;

    fn index(&self, index: B) -> &Node<P> {
        self.dag.get(*index.borrow()).unwrap()
    }
}

impl<P: PtrTrait, B: Borrow<Ptr<P>>> IndexMut<B> for Dag<P> {
    fn index_mut(&mut self, index: B) -> &mut Node<P> {
        self.dag.get_mut(*index.borrow()).unwrap()
    }
}

impl<P: PtrTrait> Dag<P> {
    /// Constructs a directed acyclic graph from the leaf sinks of a mimicking
    /// version. The optional `note`s should be included in the DAG reachable
    /// from the `leaves`, and should be `Opaque` if they should remain
    /// unmutated through optimizations.
    ///
    /// If an error occurs, the DAG (which may be in an unfinished or completely
    /// broken state) is still returned along with the error enum, so that debug
    /// tools like `render_to_svg_file` can be used.
    pub fn new(leaves: &[Rc<State>], noted: &[Rc<State>]) -> (Self, Result<(), EvalError>) {
        let mut res = Self {
            dag: Arena::new(),
            noted: vec![],
            visit_gen: 0,
        };
        let err = res.add_group(leaves, noted, None);
        (res, err)
    }

    /// All of the `note`s should be reachable from the `leaves`. If `added` is
    /// supplied then new nodes will be added to it. Note: the leaves and
    /// all their preceding nodes should not share with previously existing
    /// nodes in this DAG or else there will be duplication.
    pub fn add_group(
        &mut self,
        leaves: &[Rc<State>],
        noted: &[Rc<State>],
        mut added: Option<&mut Vec<Ptr<P>>>,
    ) -> Result<(), EvalError> {
        // keeps track if a mimick node is already tracked in the arena
        let mut lowerings: HashMap<PtrEqRc, Ptr<P>> = HashMap::new();
        // DFS from leaves to avoid much allocation, but we need the hashmap to avoid
        // retracking
        let mut path: Vec<(usize, Ptr<P>, PtrEqRc)> = vec![];
        for leaf in leaves {
            let enter_loop = match lowerings.entry(PtrEqRc(Rc::clone(leaf))) {
                Entry::Occupied(_) => false,
                Entry::Vacant(v) => {
                    let n = Node {
                        nzbw: leaf.nzbw,
                        ..Default::default()
                    };
                    let p = self.dag.insert(n);
                    v.insert(p);
                    if let Some(ref mut v) = added {
                        v.push(p);
                    }
                    path.push((0, p, PtrEqRc(Rc::clone(leaf))));
                    true
                }
            };
            if enter_loop {
                loop {
                    let (current_i, current_p, current_rc) = path.last().unwrap();
                    let u_ops = current_rc.0.op.operands();
                    if let Some(t) = Op::translate_root(&current_rc.0.op) {
                        // reached a root
                        self[current_p].op = t;
                        self[current_p].nzbw = current_rc.0.nzbw;
                        path.pop().unwrap();
                        if let Some((i, ..)) = path.last_mut() {
                            *i += 1;
                        } else {
                            break
                        }
                    } else if *current_i >= u_ops.len() {
                        // all operands should be ready
                        self[current_p].op = Op::translate(
                            &current_rc.0.op,
                            |lhs: &mut [Ptr<P>], rhs: &[Rc<State>]| {
                                for (lhs, rhs) in lhs.iter_mut().zip(rhs.iter()) {
                                    *lhs = lowerings[&PtrEqRc(Rc::clone(rhs))];
                                }
                            },
                        );
                        self[current_p].nzbw = current_rc.0.nzbw;
                        path.pop().unwrap();
                        if let Some((i, ..)) = path.last_mut() {
                            *i += 1;
                        } else {
                            break
                        }
                    } else {
                        // check next operand
                        match lowerings
                            .entry(PtrEqRc(Rc::clone(&current_rc.0.op.operands()[*current_i])))
                        {
                            Entry::Occupied(o) => {
                                // already explored
                                self[o.get()].rc += 1;
                                if let Some((i, ..)) = path.last_mut() {
                                    *i += 1;
                                } else {
                                    break
                                }
                            }
                            Entry::Vacant(v) => {
                                let mut n = Node::default();
                                n.rc += 1;
                                let p = self.dag.insert(n);
                                v.insert(p);
                                if let Some(ref mut v) = added {
                                    v.push(p);
                                }
                                path.push((
                                    0,
                                    p,
                                    PtrEqRc(Rc::clone(&current_rc.0.op.operands()[*current_i])),
                                ));
                            }
                        }
                    }
                }
            }
        }
        // handle the noted
        let mut err = Ok(());
        for (i, root) in noted.iter().enumerate() {
            match lowerings.entry(PtrEqRc(Rc::clone(root))) {
                Entry::Occupied(o) => {
                    self[o.get()].rc += 1;
                    self.noted.push(*o.get());
                }
                Entry::Vacant(_) => {
                    err = Err(EvalError::OtherString(format!(
                        "note {} is not included in DAG reached by the leaves",
                        i
                    )));
                }
            }
        }
        err
    }

    pub fn verify_integrity(&mut self) -> Result<(), EvalError> {
        for v in self.dag.vals() {
            if let Some(ref err) = v.err {
                return Err(err.clone())
            }
        }
        for p in &self.noted {
            if self.dag.get(*p).is_none() {
                return Err(EvalError::InvalidPtr)
            }
        }
        // TODO there should be more checks
        Ok(())
    }

    /// Returns a list of pointers to all nodes in no particular order
    pub fn ptrs(&self) -> Vec<Ptr<P>> {
        self.dag.ptrs().collect()
    }

    /// Assumes `ptr` is a literal
    pub fn lit(&self, ptr: Ptr<P>) -> &Bits {
        if let Literal(ref lit) = self[ptr].op {
            lit
        } else {
            panic!("node was not a literal")
        }
    }

    /// Assumes `ptr` is a literal. Returns `None` if the literal does not have
    /// bitwidth 1.
    pub fn bool(&self, ptr: Ptr<P>) -> Result<bool, EvalError> {
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
    pub fn usize(&self, ptr: Ptr<P>) -> Result<usize, EvalError> {
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

    pub fn get_bw<B: Borrow<Ptr<P>>>(&self, ptr: B) -> Result<NonZeroUsize, EvalError> {
        if let Some(w) = self[ptr].nzbw {
            Ok(w)
        } else {
            Err(EvalError::NonStaticBitwidth)
        }
    }

    /// Forbidden meta pseudo-DSL techniques in which the node at `ptr` is
    /// replaced by a set of lowered `Rc<State>` nodes with interfaces `output`
    /// and `operands` corresponding to the original interfaces.
    pub fn graft(
        &mut self,
        ptr: Ptr<P>,
        list: &mut Vec<Ptr<P>>,
        output_and_operands: &[Rc<State>],
    ) -> Result<(), EvalError> {
        if (self[ptr].op.num_operands() + 1) != output_and_operands.len() {
            return Err(EvalError::WrongNumberOfOperands)
        }
        for (i, op) in self[ptr].op.operands().iter().enumerate() {
            if self[op].nzbw != output_and_operands[i + 1].nzbw {
                return Err(EvalError::OtherString(format!(
                    "operand {}: a bitwidth of {:?} is trying to be grafted to a bitwidth of {:?}",
                    i,
                    output_and_operands[i + 1].nzbw,
                    self[op].nzbw
                )))
            }
            if !output_and_operands[i + 1].op.is_opaque() {
                return Err(EvalError::ExpectedOpaque)
            }
        }
        if self[ptr].nzbw != output_and_operands[0].nzbw {
            return Err(EvalError::WrongBitwidth)
        }
        // get length before adding group, the output node we remove will be put at this
        // address
        let list_len = list.len();
        let err = self.add_group(
            &[Rc::clone(&output_and_operands[0])],
            output_and_operands,
            Some(list),
        );
        //dag.render_to_svg_file(std::path::PathBuf::from("debug.svg")).unwrap();
        err?;
        self.verify_integrity()?;
        let start = self.noted.len() - output_and_operands.len();
        // graft inputs
        for i in 0..(output_and_operands.len() - 1) {
            let grafted = self.noted[start + i + 1];
            let graftee = self[ptr].op.operands()[i];
            // change the grafted `Opaque` to a `Copy` that routes to the graftee instead of
            // needing to change all the operands of potentially many internal nodes.

            self[grafted].op = Copy([graftee]);
        }
        // graft output
        let output_p = self.noted[start];
        let output_node = self.dag.remove(output_p).unwrap();
        assert_eq!(list.swap_remove(list_len), output_p);
        let old_output = self.dag.replace_and_keep_gen(ptr, output_node).unwrap();
        // preserve original reference count
        self[ptr].rc = old_output.rc;
        // relist because there are cases where this node needs to be reprocessed
        list.push(ptr);
        // remove the temporary noted nodes
        self.noted.drain(start..);
        // this is very important to prevent infinite cycles where literals are not
        // being propogated and eliminating nodes
        self.eval_tree(ptr)?;
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
