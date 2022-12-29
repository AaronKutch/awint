// note: have this module not be named "assert" to avoid clashes

use crate::{common::register_assertion_bit, dag};

#[doc(hidden)]
pub fn internal_assert(assert_true: impl Into<dag::bool>) {
    register_assertion_bit(assert_true.into())
}

#[doc(hidden)]
#[track_caller]
pub fn internal_assert_eq<AsRefBitsType: AsRef<dag::Bits>>(lhs: AsRefBitsType, rhs: AsRefBitsType) {
    let eq = if let dag::Some(eq) = lhs.as_ref().const_eq(rhs.as_ref()) {
        eq
    } else {
        panic!("`assert_eq` failed for `lhs` and `rhs` because they have different bitwidths")
    };
    register_assertion_bit(eq)
}

#[doc(hidden)]
#[track_caller]
pub fn internal_assert_ne<AsRefBitsType: AsRef<dag::Bits>>(lhs: AsRefBitsType, rhs: AsRefBitsType) {
    let ne = if let dag::Some(ne) = lhs.as_ref().const_ne(rhs.as_ref()) {
        ne
    } else {
        panic!("`assert_ne` failed for `lhs` and `rhs` because they have different bitwidths")
    };
    register_assertion_bit(ne)
}

/// Mimicking `assert` that takes `awi::bool` or `dag::bool`
#[macro_export]
macro_rules! assert {
    ($assert_true:expr) => {
        $crate::internal_assert($assert_true)
    };
}

/// Mimicking `assert_eq` that takes inputs of `AsRef<dag::Bits>`
#[macro_export]
macro_rules! assert_eq {
    ($lhs:expr, $rhs:expr) => {
        $crate::internal_assert_eq($lhs, $rhs)
    };
}

/// Mimicking `assert_ne` that takes inputs of `AsRef<dag::Bits>`
#[macro_export]
macro_rules! assert_ne {
    ($lhs:expr, $rhs:expr) => {
        $crate::internal_assert_ne($lhs, $rhs)
    };
}
