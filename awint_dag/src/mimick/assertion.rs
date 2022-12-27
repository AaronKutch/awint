// note: have this module not be named "assert" to avoid clashes

use crate::{common::register_assertion_bit, dag};

pub fn internal_assert(assert_true: impl Into<dag::bool>) {
    register_assertion_bit(assert_true.into())
}

#[track_caller]
pub fn internal_assert_eq(lhs: impl AsRef<dag::Bits>, rhs: impl AsRef<dag::Bits>) {
    let eq = if let Some(eq) = lhs.as_ref().const_eq(rhs.as_ref()) {
        eq
    } else {
        panic!("`assert_eq` failed for `lhs` and `rhs` because they have different bitwidths")
    };
    register_assertion_bit(eq)
}

#[track_caller]
pub fn internal_assert_ne(lhs: impl AsRef<dag::Bits>, rhs: impl AsRef<dag::Bits>) {
    let ne = if let Some(ne) = lhs.as_ref().const_ne(rhs.as_ref()) {
        ne
    } else {
        panic!("`assert_ne` failed for `lhs` and `rhs` because they have different bitwidths")
    };
    register_assertion_bit(ne)
}

#[macro_export]
macro_rules! assert {
    ($assert_true:expr) => {
        $crate::internal_assert($assert_true)
    };
}

#[macro_export]
macro_rules! assert_eq {
    ($lhs:expr, $rhs:expr) => {
        $crate::internal_assert_eq($lhs, $rhs)
    };
}

#[macro_export]
macro_rules! assert_ne {
    ($lhs:expr, $rhs:expr) => {
        $crate::internal_assert_ne($lhs, $rhs)
    };
}
