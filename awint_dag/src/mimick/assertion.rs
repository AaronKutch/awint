// note: have this module not be named "assert" to avoid clashes

pub use core::stringify;

use awint_ext::awint_internals::Location;

use crate::{dag, epoch::register_assertion_bit_for_current_epoch, Lineage, Op};

/// Creates an assertion, returning `None` if eager evaluation can determine
/// that it is false
pub fn create_assertion(assert_true: dag::bool, location: Location) -> Option<()> {
    let p_state = assert_true.state();
    if let Op::Literal(ref lit) = p_state.get_op() {
        assert_eq!(lit.bw(), 1);
        if lit.to_bool() {
            Some(())
        } else {
            None
        }
    } else {
        register_assertion_bit_for_current_epoch(assert_true, location);
        Some(())
    }
}

#[doc(hidden)]
#[track_caller]
pub fn internal_assert(stringified: &str, assert_true: impl Into<dag::bool>, location: Location) {
    if create_assertion(assert_true.into(), location).is_none() {
        panic!(
            "`awint_dag::assert({stringified})` failed because eager evaluation determined that \
             the value is false"
        )
    }
}

#[doc(hidden)]
#[track_caller]
pub fn internal_assert_eq<AsRefBitsType: AsRef<dag::Bits>>(
    lhs_stringified: &str,
    rhs_stringified: &str,
    lhs: AsRefBitsType,
    rhs: AsRefBitsType,
    location: Location,
) {
    let lhs = lhs.as_ref();
    let rhs = rhs.as_ref();
    if let dag::Some(eq) = lhs.const_eq(rhs) {
        if create_assertion(eq, location).is_none() {
            if let (Op::Literal(lhs_lit), Op::Literal(rhs_lit)) =
                (lhs.state().get_op(), rhs.state().get_op())
            {
                panic!(
                    "`awint_dag::assert_eq(\n {lhs_stringified},\n {rhs_stringified}\n)` failed \
                     because eager evaluation determined that these are unequal:\n lhs: \
                     {lhs_lit}\n rhs: {rhs_lit}"
                )
            } else {
                panic!(
                    "`awint_dag::assert_eq(\n {lhs_stringified},\n {rhs_stringified}\n)` failed \
                     because eager evaluation determined that these are unequal"
                )
            }
        }
    } else {
        panic!(
            "`awint_dag::assert_eq` failed because of unequal bitwidths\n lhs.bw(): {}\n \
             rhs.bw(): {}",
            lhs.bw(),
            rhs.bw()
        )
    }
}

#[doc(hidden)]
#[track_caller]
pub fn internal_assert_ne<AsRefBitsType: AsRef<dag::Bits>>(
    lhs_stringified: &str,
    rhs_stringified: &str,
    lhs: AsRefBitsType,
    rhs: AsRefBitsType,
    location: Location,
) {
    let lhs = lhs.as_ref();
    let rhs = rhs.as_ref();
    if let dag::Some(ne) = lhs.const_ne(rhs) {
        if create_assertion(ne, location).is_none() {
            if let (Op::Literal(lhs_lit), Op::Literal(rhs_lit)) =
                (lhs.state().get_op(), rhs.state().get_op())
            {
                panic!(
                    "`awint_dag::assert_ne(\n {lhs_stringified},\n {rhs_stringified}\n)` failed \
                     because eager evaluation determined that these are equal:\n lhs: {lhs_lit}\n \
                     rhs: {rhs_lit}"
                )
            } else {
                panic!(
                    "`awint_dag::assert_ne(\n {lhs_stringified},\n {rhs_stringified}\n)` failed \
                     because eager evaluation determined that these are equal"
                )
            }
        }
    } else {
        // this is deliberate
        panic!(
            "`awint_dag::assert_ne` failed because of unequal bitwidths\n lhs.bw(): {}\n \
             rhs.bw(): {}",
            lhs.bw(),
            rhs.bw()
        )
    }
}

/// Mimicking `assert` that takes `awi::bool` or `dag::bool`
#[macro_export]
macro_rules! assert {
    ($assert_true:expr) => {
        $crate::internal_assert(
            $crate::stringify!($assert_true),
            $assert_true,
            $crate::location!(),
        )
    };
}

/// Mimicking `assert_eq` that takes inputs of `AsRef<dag::Bits>`
#[macro_export]
macro_rules! assert_eq {
    ($lhs:expr, $rhs:expr) => {
        $crate::internal_assert_eq(
            $crate::stringify!($lhs),
            $crate::stringify!($rhs),
            $lhs,
            $rhs,
            $crate::location!(),
        )
    };
}

/// Mimicking `assert_ne` that takes inputs of `AsRef<dag::Bits>`. This checks
/// for bitwidth equality.
#[macro_export]
macro_rules! assert_ne {
    ($lhs:expr, $rhs:expr) => {
        $crate::internal_assert_ne(
            $crate::stringify!($lhs),
            $crate::stringify!($rhs),
            $lhs,
            $rhs,
            $crate::location!(),
        )
    };
}
