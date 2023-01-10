// note: have this module not be named "assert" to avoid clashes

use awint_ext::awint_internals::Location;

use crate::{common::register_assertion_bit, dag};

#[doc(hidden)]
pub fn internal_assert(assert_true: impl Into<dag::bool>, location: Location) {
    register_assertion_bit(assert_true.into(), location)
}

#[doc(hidden)]
#[track_caller]
pub fn internal_assert_eq<AsRefBitsType: AsRef<dag::Bits>>(
    lhs: AsRefBitsType,
    rhs: AsRefBitsType,
    location: Location,
) {
    let eq = if let dag::Some(eq) = lhs.as_ref().const_eq(rhs.as_ref()) {
        eq
    } else {
        panic!("`assert_eq` failed for `lhs` and `rhs` because they have different bitwidths")
    };
    register_assertion_bit(eq, location)
}

#[doc(hidden)]
#[track_caller]
pub fn internal_assert_ne<AsRefBitsType: AsRef<dag::Bits>>(
    lhs: AsRefBitsType,
    rhs: AsRefBitsType,
    location: Location,
) {
    let ne = if let dag::Some(ne) = lhs.as_ref().const_ne(rhs.as_ref()) {
        ne
    } else {
        panic!("`assert_ne` failed for `lhs` and `rhs` because they have different bitwidths")
    };
    register_assertion_bit(ne, location)
}

/// Mimicking `assert` that takes `awi::bool` or `dag::bool`
#[macro_export]
macro_rules! assert {
    ($assert_true:expr) => {
        $crate::internal_assert($assert_true, $crate::location!())
    };
}

/// Mimicking `assert_eq` that takes inputs of `AsRef<dag::Bits>`
#[macro_export]
macro_rules! assert_eq {
    ($lhs:expr, $rhs:expr) => {
        $crate::internal_assert_eq($lhs, $rhs, $crate::location!())
    };
}

/// Mimicking `assert_ne` that takes inputs of `AsRef<dag::Bits>`
#[macro_export]
macro_rules! assert_ne {
    ($lhs:expr, $rhs:expr) => {
        $crate::internal_assert_ne($lhs, $rhs, $crate::location!())
    };
}
