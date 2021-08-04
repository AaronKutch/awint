//! This DAG is for lowering into a LUT-only DAG

use std::{
    collections::{hash_map::Entry, HashMap},
    rc::Rc,
};

use crate::{
    lowering::{Arena, Dag, Node, Ptr, PtrEqRc},
    mimick,
};

impl Dag {
    /// Constructs a directed acyclic graph from the leaf sinks of a mimicking
    /// version
    pub fn new(leaves: Vec<Rc<mimick::State>>) -> Self {
        // keeps track if a mimick node is already tracked in the arena
        let mut lowerings: HashMap<PtrEqRc, Ptr> = HashMap::new();
        // used later for when all nodes are allocated
        let mut raisings: Vec<(Ptr, PtrEqRc)> = Vec::new();
        // keep a frontier which will guarantee that the whole mimick DAG is explored,
        // and keep track of dependents
        let mut frontier = leaves;
        let mut dag: Arena<Node> = Arena::new();
        // because some nodes may not be in the arena yet, we have to bootstrap
        // dependencies by looking up the source later (source, sink)
        let mut deps: Vec<(PtrEqRc, Ptr)> = Vec::new();
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
        Self { dag }
    }

    /// Returns a list of pointers to all nodes in no particular order
    pub fn list_ptrs(&self) -> Vec<Ptr> {
        self.dag.list_ptrs()
    }

    /// Returns all source roots that have no operands
    pub fn roots(&self) -> Vec<Ptr> {
        let mut v = Vec::new();
        for p in self.list_ptrs() {
            if self.dag[p].ops.is_empty() {
                v.push(p);
            }
        }
        v
    }

    /// Returns all sink leaves that have no dependents
    pub fn leaves(&self) -> Vec<Ptr> {
        let mut v = Vec::new();
        for p in self.list_ptrs() {
            if self.dag[p].deps.is_empty() {
                v.push(p);
            }
        }
        v
    }
}