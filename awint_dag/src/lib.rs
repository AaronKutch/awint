//! # DAG functionality for `awint`
//!
//! **NOTE**: this crate is usable but design choices are still in flux.
//!
//! This crate is intended to be used as a reexport from the `awint` crate with
//! the "dag" feature flag enabled.
//!
//! Outside of the core functionality which will be useful for big integer
//! arithmetic in constrained environments, there is a secondary goal with this
//! system of `awint` crates to create a new kind of RTL description library
//! that is not a DSL but is rather plain Rust code that can be run normally.
//! This `awint_dag` crate supplies a "mimicking" structs with the same names as
//! their counterparts in `awint::awi::*`, the difference being that they
//! have purely lazy execution, creating a DAG recording the order in which
//! different `Bits` operations are applied.
//!
//! ```
//! // In the future we may have a macro that can duplicate the code into a
//! // module that has `awint::awi` imported, and another module that has
//! // `awint::dag` imported, so that you can have a normal running version
//! // of the code and the DAG recording version at the same time. But for
//! // research for now, we add a flag to our crate that switches between
//! // the types, so that we can rapidly switch between for development.
//!
//! //#[cfg(feature = "dag")]
//! use awint::dag::*;
//! //#![cfg(not(feature = "dag"))]
//! //use awint::awi::*;
//!
//! // This is just some arbitrary example I coded up, note that you can use
//! // almost all of Rust's features that you can use on the normal types
//! struct StateMachine(inlawi_ty!(16));
//! impl StateMachine {
//!     pub fn new() -> Self {
//!         Self(inlawi!(0u16))
//!     }
//!
//!     //#[cfg(feature = "dag")]
//!     //pub fn state(&self) -> PState {
//!     //    use awint::awint_dag::Lineage;
//!     //    self.0.state()
//!     //}
//!
//!     pub fn update(&mut self, input: inlawi_ty!(4)) -> Option<inlawi_ty!(4)> {
//!         let mut s0 = inlawi!(0u4);
//!         let mut s1 = inlawi!(0u4);
//!         let mut s2 = inlawi!(0u4);
//!         let mut s3 = inlawi!(0u4);
//!         cc!(self.0; s3, s2, s1, s0)?;
//!         s2.xor_(&s0)?;
//!         s3.xor_(&s1)?;
//!         s1.xor_(&s2)?;
//!         s0.xor_(&s3)?;
//!         s3.rotl_(1)?;
//!         s2.mux_(&input, s0.get(2)?)?;
//!         cc!(s3, s2, s1, s0; self.0)?;
//!         Some(s2)
//!     }
//! }
//!
//! let mut m = StateMachine::new();
//! let _ = m.update(InlAwi::opaque()).unwrap();
//! let out = m.update(inlawi!(0110)).unwrap();
//!
//! //#[cfg(feature = "dag")]
//! //{
//!     use awint::awint_dag::{OpDag, Lineage};
//!     let noted = [out.state()];
//!     let (mut graph, res) = OpDag::new(&noted, &noted);
//!
//!     // will do basic evaluations on DAGs
//!     graph.eval_all_noted().unwrap();
//!
//!     dbg!(&graph);
//!
//!     // The graphs unfortunately get ugly really fast, but you mainly want
//!     // to use these for developing general algorithms on simple examples.
//!     // This is available with the "debug" feature flag on `awint_dag`
//!     //graph
//!     //    .render_to_svg_file(std::path::PathBuf::from("rendered.svg"))
//!     //    .unwrap();
//!     res.unwrap();
//!
//!     // lower into purely static copies, gets, sets, and lookup tables.
//!     // You will want to design algorithms on the resulting `OpDag` or
//!     // further translate into another form.
//!     graph.lower_all_noted().unwrap();
//!
//!     for node in graph.a.vals() {
//!         use awint::awint_dag::Op::*;
//!         core::assert!(matches!(
//!             node.op,
//!             Opaque(_)
//!                 | Literal(_)
//!                 | Copy(_)
//!                 | StaticGet(_, _)
//!                 | StaticSet(_, _)
//!                 | StaticLut(_, _)
//!         ));
//!     }
//! //}
//! ```
//!
//! ## Important Notes
//!
//! - If you try to use a mimicking `bool` in an `if` statement or with some
//!   binary operations you will get an error. `Bits::mux_` can be used to
//!   conditionally merge the result of two computational paths instead of using
//!   `if` statements.
//!
//!   ```
//!   //use awint::awi::*;
//!   use awint::dag::*;
//!
//!   let mut lhs = inlawi!(zero: ..8);
//!   let rhs = inlawi!(umax: ..8);
//!   let x = inlawi!(10101010);
//!   // make use of the primitive conversion functions to do things
//!   let mut y = InlAwi::from_u8(0);
//!   y.opaque_();
//!
//!   // error: expected `bool`, found struct `bool`
//!   //if lhs.ult(&rhs).unwrap() {
//!   //    lhs.xor_(&x).unwrap();
//!   //} else {
//!   //    lhs.lshr_(y.to_usize()).unwrap();
//!   //};
//!
//!   // a little more cumbersome, but we get to use all the features of
//!   // normal Rust in metaprogramming and don't have to support an entire DSL
//!
//!   let mut tmp0 = inlawi!(lhs; ..8).unwrap();
//!   tmp0.xor_(&x).unwrap();
//!   let mut tmp1 = inlawi!(lhs; ..8).unwrap();
//!   tmp1.lshr_(y.to_usize()).unwrap();
//!
//!   let lt = lhs.ult(&rhs).unwrap();
//!   lhs.mux_(&tmp0, lt).unwrap();
//!   lhs.mux_(&tmp1, !lt).unwrap();
//!   ```
//!
//! - The mimicking types have an extra `opaque` constructor kind that has no
//!   definitive bit pattern. This can be accessed through `Bits::opaque_`,
//!   `ExtAwi::opaque(w)`, `InlAwi::opaque()`, and in macros like
//!   `inlawi!(opaque: ..8)`. This is useful for placeholder values in
//!   algorithms that prevents evaluation from doing anything with the sink tree
//!   of these values.
//! - The macros from `awint_dag` use whatever `usize`, `ExtAwi`, `InlAwi`, and
//!   `Bits` structs are imported in their scope. If you are mixing regular and
//!   mimicking types and are getting name collisions in macros, you can glob
//!   import `awint::awi::*` or `awint::dag::*` in the same scope as the macro
//!   (or add an extra block scope around the macro to glob import in), which
//!   should fix the errors.
//! - There are generation counters on the `PState`s that are enabled when debug
//!   assertions are on
//! - In long running programs that are generating a lot of separate DAGs, you
//!   should use `StateEpoch`s to clear up the thread local data that records
//!   DAGs.

#![deny(unsafe_op_in_unsafe_fn)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::comparison_chain)]
#![cfg_attr(feature = "try_support", feature(try_trait_v2))]
#![cfg_attr(feature = "try_support", feature(try_trait_v2_residual))]
#![cfg_attr(feature = "try_support", feature(never_type))]

mod common;
pub mod lowering;
pub mod mimick;
pub use awint_ext::awint_internals::location;
pub use awint_macro_internals::triple_arena;
#[cfg(feature = "debug")]
pub use awint_macro_internals::triple_arena_render;
pub use common::{EvalError, EvalResult, Lineage, Op, PNode, PState, State, StateEpoch};
pub use lowering::{OpDag, OpNode};
pub use mimick::{
    assertion::{internal_assert, internal_assert_eq, internal_assert_ne},
    primitive,
};

/// Raw access to thread-local `State` related things
pub mod state {
    pub use crate::common::{
        clear_thread_local_state, next_state_visit_gen, StateEpoch, EPOCH_GEN, EPOCH_STACK,
        STATE_ARENA, STATE_VISIT_GEN,
    };
}

pub use crate::mimick::{Bits, ExtAwi, InlAwi};

/// All mimicking items
pub mod dag {
    pub use awint_ext::bw;

    pub use crate::{
        mimick::{
            assert, assert_eq, assert_ne, Bits, ExtAwi, InlAwi, Option,
            Option::{None, Some},
            Result,
            Result::{Err, Ok},
        },
        primitive::*,
    };
}
