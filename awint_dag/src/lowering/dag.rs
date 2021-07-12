//! This DAG is for lowering into a LUT-only DAG

use std::{collections::HashMap, num::NonZeroUsize, rc::Rc};

use awint_ext::ExtAwi;

use crate::{
    lowering::{Arena, Op, Ptr},
    mimick,
};

struct Node {
    bw: NonZeroUsize,
    op: Op,
    // When deleting nodes, we need to make sure that no other nodes are depending on this one
    dependents: Vec<Ptr>,
}

struct Dag {
    dag: Arena<Node>,
    /// keeps track if a mimick node is already in the arena
    rc_nodes: HashMap<Rc<mimick::Op>, Ptr>,
}
