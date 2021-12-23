//! This DAG is for lowering into a LUT-only DAG

use std::{
    collections::{hash_map::Entry, HashMap},
    ops::{Index, IndexMut},
    rc::Rc,
};

use triple_arena::{Arena, Ptr};

use super::EvalError;
use crate::{
    lowering::{Dag, Node, PtrEqRc, P0},
    mimick,
};

impl Index<Ptr<P0>> for Dag {
    type Output = Node;

    fn index(&self, index: Ptr<P0>) -> &Node {
        self.dag.get(index).unwrap()
    }
}

impl IndexMut<Ptr<P0>> for Dag {
    fn index_mut(&mut self, index: Ptr<P0>) -> &mut Node {
        self.dag.get_mut(index).unwrap()
    }
}

impl Index<&Ptr<P0>> for Dag {
    type Output = Node;

    fn index(&self, index: &Ptr<P0>) -> &Node {
        self.dag.get(*index).unwrap()
    }
}

impl IndexMut<&Ptr<P0>> for Dag {
    fn index_mut(&mut self, index: &Ptr<P0>) -> &mut Node {
        self.dag.get_mut(*index).unwrap()
    }
}

impl Dag {
    /// Constructs a directed acyclic graph from the leaf sinks of a mimicking
    /// version
    pub fn new(leaves: Vec<Rc<mimick::State>>) -> Self {
        // keeps track if a mimick node is already tracked in the arena
        let mut lowerings: HashMap<PtrEqRc, Ptr<P0>> = HashMap::new();
        // used later for when all nodes are allocated
        let mut raisings: Vec<(Ptr<P0>, PtrEqRc)> = Vec::new();
        // keep a frontier which will guarantee that the whole mimick DAG is explored,
        // and keep track of dependents
        let mut frontier = leaves;
        let mut dag: Arena<P0, Node> = Arena::new();
        // because some nodes may not be in the arena yet, we have to bootstrap
        // dependencies by looking up the source later (source, sink)
        let mut deps: Vec<(PtrEqRc, Ptr<P0>)> = Vec::new();
        while let Some(next) = frontier.pop() {
            match lowerings.entry(PtrEqRc(Rc::clone(&next))) {
                Entry::Occupied(_) => (),
                Entry::Vacant(v) => {
                    let sink = dag.insert(Node {
                        nzbw: next.nzbw,
                        op: next.op.clone(),
                        ops: Vec::new(),
                        deps: Vec::new(),
                    });
                    v.insert(sink);
                    for source in next.ops.clone() {
                        deps.push((PtrEqRc(Rc::clone(&source)), sink));
                        frontier.push(source);
                    }
                    raisings.push((sink, PtrEqRc(Rc::clone(&next))));
                }
            }
        }
        // set all dependents
        for dep in &deps {
            dag[lowerings[&dep.0]].deps.push(dep.1);
        }
        // set all operands
        for (ptr, rc) in raisings {
            for i in 0..rc.0.ops.len() {
                dag[ptr].ops.push(lowerings[&PtrEqRc(rc.0.ops[i].clone())]);
            }
        }
        let dag = Self { dag };
        dag.verify_integrity().unwrap();
        dag
    }

    /// Checks that the DAG is not broken and that the bitwidth checks work.
    /// Note that if the DAG is evaluated, there may be invalid operand value
    /// errors.
    pub fn verify_integrity(&self) -> Result<(), EvalError> {
        'outer: for p in self.list_ptrs() {
            let len = self[p].ops.len();
            if let Some(expected) = self[p].op.operands_len() {
                if expected != len {
                    return Err(EvalError::WrongNumberOfOperands)
                }
            }
            let mut bitwidths = vec![];
            for i in 0..self[p].ops.len() {
                let op = self[p].ops[i];
                if let Some(nzbw) = self[op].nzbw {
                    bitwidths.push(nzbw.get());
                } else {
                    // can't do anything
                    continue 'outer
                }
                if !self[op].deps.contains(&p) {
                    return Err(EvalError::Other)
                }
            }
            if let Some(nzbw) = self[p].nzbw {
                if self[p].op.check_bitwidths(nzbw.get(), &bitwidths) {
                    return Err(EvalError::WrongBitwidth)
                }
            }
        }
        Ok(())
    }

    /// Returns a list of pointers to all nodes in no particular order
    pub fn list_ptrs(&self) -> Vec<Ptr<P0>> {
        let mut v = vec![];
        for (p, _) in &self.dag {
            v.push(p);
        }
        v
    }

    /// Returns all source roots that have no operands
    pub fn roots(&self) -> Vec<Ptr<P0>> {
        let mut v = Vec::new();
        for p in self.list_ptrs() {
            if self[p].ops.is_empty() {
                v.push(p);
            }
        }
        v
    }

    /// Returns all sink leaves that have no dependents
    pub fn leaves(&self) -> Vec<Ptr<P0>> {
        let mut v = Vec::new();
        for p in self.list_ptrs() {
            if self[p].deps.is_empty() {
                v.push(p);
            }
        }
        v
    }
}
