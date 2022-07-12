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
    pub leaves: Vec<Ptr<P>>,
    pub noted_roots: Vec<Ptr<P>>,
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
    /// version. The optional `roots` make the `Dag` keep track of select nodes
    /// in `noted_roots`, which must be `Opaque` nodes.
    pub fn new(leaves: &[Rc<State>], roots: &[Rc<State>]) -> (Self, Result<(), EvalError>) {
        let mut res = Self {
            dag: Arena::new(),
            leaves: vec![],
            noted_roots: vec![],
        };
        let err = res.add_group(leaves, roots);
        (res, err)
    }

    /// All of the `roots` should be `Opaque`s. `roots` does not have to contain
    /// all actual roots, just roots that need to be noted and preserved through
    /// optimizations. All of the `roots` should be include at least one of the
    /// `leaves` in their DAG. Note: the leaves and all their preceding
    /// nodes should not share with previously existing nodes in this DAG or
    /// else there will be duplication.
    pub fn add_group(
        &mut self,
        leaves: &[Rc<State>],
        roots: &[Rc<State>],
    ) -> Result<(), EvalError> {
        // keeps track if a mimick node is already tracked in the arena
        let mut lowerings: HashMap<PtrEqRc, Ptr<P>> = HashMap::new();
        // DFS from leaves to avoid much allocation, but we need the hashmap to avoid
        // retracking
        let mut path: Vec<(usize, Ptr<P>, PtrEqRc)> = vec![];
        for leaf in leaves {
            let mut enter_loop = false;
            match lowerings.entry(PtrEqRc(Rc::clone(leaf))) {
                Entry::Occupied(_) => (),
                Entry::Vacant(v) => {
                    let p = self.dag.insert(Node::default());
                    v.insert(p);
                    path.push((0, p, PtrEqRc(Rc::clone(leaf))));
                    enter_loop = true;
                }
            }
            if enter_loop {
                loop {
                    let (current_i, current_p, current_rc) = path.last().unwrap();
                    if let Some(t) = Op::translate_root(&self[current_p].op) {
                        // reached a root
                        self[current_p].op = t;
                        path.pop().unwrap();
                        if let Some((i, ..)) = path.last_mut() {
                            *i += 1;
                        } else {
                            break
                        }
                    } else {
                        let t_ops = self[current_p].op.operands();
                        if *current_i >= t_ops.len() {
                            // all operands should be ready
                            self[current_p].op = Op::translate(
                                &current_rc.0.op,
                                |lhs: &mut [Ptr<P>], rhs: &[Rc<State>]| {
                                    for (lhs, rhs) in lhs.iter_mut().zip(rhs.iter()) {
                                        *lhs = lowerings[&PtrEqRc(Rc::clone(rhs))];
                                    }
                                },
                            );
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
                                Entry::Occupied(_) => {
                                    // already explored
                                    path.pop().unwrap();
                                    if let Some((i, ..)) = path.last_mut() {
                                        *i += 1;
                                    } else {
                                        break
                                    }
                                }
                                Entry::Vacant(v) => {
                                    let p = self.dag.insert(Node::default());
                                    v.insert(p);
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
        }
        // handle noted roots
        let mut err = Ok(());
        for (i, root) in roots.iter().enumerate() {
            match lowerings.entry(PtrEqRc(Rc::clone(root))) {
                Entry::Occupied(o) => {
                    if !matches!(self.dag[o.get()].op, Opaque(_)) {
                        self.dag[o.get()].err = Some(EvalError::ExpectedOpaque);
                        err = Err(EvalError::ExpectedOpaque);
                    }
                    self.noted_roots.push(*o.get());
                }
                Entry::Vacant(_) => {
                    err = Err(EvalError::OtherString(format!(
                        "root {} is not included in DAG reached by the leaves",
                        i
                    )));
                }
            }
        }
        err
    }

    /// Checks that the DAG is not broken and that the bitwidth checks work.
    /// Note that if the DAG is evaluated, there may be invalid operand value
    /// errors.
    pub fn verify_integrity(&mut self) -> Result<(), EvalError> {
        for p in self.leaves() {
            if self.dag.get(*p).is_none() {
                return Err(EvalError::InvalidPtr)
            }
        }
        for p in &self.noted_roots {
            if self.dag.get(*p).is_none() {
                return Err(EvalError::InvalidPtr)
            }
        }
        for v in self.dag.vals() {
            if let Some(ref err) = v.err {
                return Err(err.clone())
            }
        }
        Ok(())
    }

    /// Returns a list of pointers to all nodes in no particular order
    pub fn ptrs(&self) -> Vec<Ptr<P>> {
        self.dag.ptrs().collect()
    }

    /// Returns all sink leaves that have no dependents
    pub fn leaves(&self) -> &[Ptr<P>] {
        &self.leaves
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

    pub fn strip_opaque_leaf<B: Borrow<Ptr<P>>>(&mut self, ptr: B) -> Result<Ptr<P>, EvalError> {
        let ptr = *ptr.borrow();
        if let Opaque(ref v) = self[ptr].op {
            if v.len() == 1 {
                let res = Ok(v[0]);
                self.dag.remove(ptr).unwrap();
                res
            } else {
                Err(EvalError::OtherString(format!(
                    "opaque leaf does not have 1 operand: {:?}",
                    self[ptr]
                )))
            }
        } else {
            Err(EvalError::OtherString(format!(
                "is not opaque leaf: {:?}",
                self[ptr]
            )))
        }
    }

    /// Forbidden meta pseudo-DSL techniques in which the node at `ptr` is
    /// replaced by a set of lowered `Rc<State>` nodes with interfaces `ops` and
    /// `deps` corresponding to the original interfaces.
    pub fn graft(
        &mut self,
        ptr: Ptr<P>,
        leaf: Rc<State>,
        roots: &[Rc<State>],
    ) -> Result<(), EvalError> {
        let err = self.add_group(&[leaf], roots);
        //dag.render_to_svg_file(std::path::PathBuf::from("debug.svg")).unwrap();
        err?;
        self.verify_integrity()?;
        //dag.eval()?; //TODO eval just part
        // TODO graft internal
        //self.graft_dag(ptr, dag)
        Ok(())
    }

    /// Always renders to file, and then returns errors
    #[cfg(feature = "debug")]
    pub fn render_to_svg_file(&mut self, out_file: std::path::PathBuf) -> Result<(), EvalError> {
        let res = self.verify_integrity();
        triple_arena_render::render_to_svg_file(&self.dag, false, out_file).unwrap();
        res
    }

    /// Always renders to file, and then returns errors
    #[cfg(feature = "debug")]
    pub fn eval_and_render_to_svg_file(
        &mut self,
        out_file: std::path::PathBuf,
    ) -> Result<(), EvalError> {
        let res0 = self.verify_integrity();
        if res0.is_err() {
            triple_arena_render::render_to_svg_file(&self.dag, false, out_file).unwrap();
            res0
        } else {
            // TODO
            //let res1 = self.eval();
            triple_arena_render::render_to_svg_file(&self.dag, false, out_file).unwrap();
            //res1
            Ok(())
        }
    }
}
