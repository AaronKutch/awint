//! Macro fuzzing tests. Uses code from `build.rs`

// Only enable on Miri, because it greatly slows down the normal debug cycle
#![cfg(miri)]
#![allow(bad_style)]
#![allow(unused_imports)]
#![allow(unused_mut)]

use std::any::Any;

use awint::awi::*;

// The macros have a highly nonlinear way of determining if something is
// infallible, and it is impractical to use plain `assert_eq` and determine
// whether to wrap the right side of an expression with `Some`. There are edge
// cases like `cc!(..4; ..4)` that the current code gen thinks is fallible, but
// a reasonable fuzzer would think is infallible. The temporary solution we use
// here is these functions, and manual test cases elsewhere to make sure that
// useful infallible cases work.
//
// TODO in the code gen refactor, we need a function to tell the fuzzer whether
// something is fallible or not. Then, we can scrap the workaround below and
// just use plain `assert_eq`.

#[track_caller]
fn eq_unit(lhs: &dyn Any, _rhs: ()) {
    if let Some(()) = lhs.downcast_ref::<()>() {
    } else if let Some(lhs) = lhs.downcast_ref::<Option<()>>() {
        // note: this has to be done this way so that `#[track_caller]` works as
        // intended
        if let None = lhs.as_ref() {
            panic!("lhs (Option<()>) is `None`")
        }
    } else {
        panic!("lhs is not a recognized type");
    }
}

#[track_caller]
fn eq_inl<const BW: usize, const LEN: usize>(lhs: &dyn Any, rhs: InlAwi<BW, LEN>) {
    if let Some(lhs) = lhs.downcast_ref::<InlAwi<BW, LEN>>() {
        assert_eq!(*lhs, rhs);
    } else if let Some(lhs) = lhs.downcast_ref::<Option<InlAwi<BW, LEN>>>() {
        if let Some(lhs) = lhs.as_ref() {
            assert_eq!(*lhs, rhs);
        } else {
            panic!("lhs (Option<InlAwi>) is `None`")
        }
    } else {
        panic!("lhs is not a recognized type");
    }
}

#[track_caller]
fn eq_ext(lhs: &dyn Any, rhs: ExtAwi) {
    if let Some(lhs) = lhs.downcast_ref::<ExtAwi>() {
        assert_eq!(*lhs, rhs);
    } else if let Some(lhs) = lhs.downcast_ref::<Option<ExtAwi>>() {
        if let Some(lhs) = lhs.as_ref() {
            assert_eq!(*lhs, rhs);
        } else {
            panic!("lhs (Option<ExtAwi>) is `None`")
        }
    } else {
        panic!("lhs is not a recognized type");
    }
}

#[track_caller]
fn eq_awi(lhs: &dyn Any, rhs: Awi) {
    if let Some(lhs) = lhs.downcast_ref::<Awi>() {
        assert_eq!(*lhs, rhs);
    } else if let Some(lhs) = lhs.downcast_ref::<Option<Awi>>() {
        if let Some(lhs) = lhs.as_ref() {
            assert_eq!(*lhs, rhs);
        } else {
            panic!("lhs (Option<Awi>) is `None`")
        }
    } else {
        panic!("lhs is not a recognized type");
    }
}

include!(concat!(env!("OUT_DIR"), "/generated.rs"));
