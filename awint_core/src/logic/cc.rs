use awint_internals::*;

use crate::Bits;

/// For the concatenations of components macros, we have to perform checks
/// in a particular way to prevent problems. One problem is that arbitrary
/// ranges need to be checked for reversal, and computation must not
/// progress or else the width computations will overflow (e.x. `start..end`
/// and `start < end` but `end - start` is used to calculate width).
///
/// The layout of the macros are like:
/// constants // define `InlAwi` constants
/// if component_checks {None} // check that ranges are valid first
/// else {
///     common_bw // calculate the common bitwidth
///     if concats_checks {None} // run common bitwidth equality check
///     else {
///         construction, fielding, returning
///     }
/// }
///
/// The component checks for ranges all consist of checking if
/// `x.bw() < end || end < start` and returning `None` if true. There are other
/// checks for other cases.
///
/// For the purposes of avoiding `if` statements with booleans for
/// `awint_dag`, reducing the number of nested statements, and avoiding
/// requiring the user to import more than `Bit`s and `InlAwi`, we have these
/// functions.
impl Bits {
    /// Used by the macros to enforce compiler warnings
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn must_use<T>(t: T) -> T {
        t
    }

    /// This is for the macros crate to plug into the `LEN` generic in
    /// `InlAwi<BW, LEN>`, because the build architecture pointer width can be
    /// different from the target architecture pointer width (and we can't use
    /// `target_pointer_width` because it corresponds to whatever architecture
    /// the procedural macro crate is running for).
    #[doc(hidden)]
    pub const fn unstable_raw_digits(bw: usize) -> usize {
        raw_digits(bw)
    }

    #[doc(hidden)]
    #[inline]
    pub const fn unstable_lt_checks<const N: usize>(lt_checks: [(usize, usize); N]) -> Option<()> {
        const_for!(i in {0..N} {
            if lt_checks[i].0 < lt_checks[i].1 {
                return None
            }
        });
        Some(())
    }

    #[doc(hidden)]
    #[inline]
    pub const fn unstable_common_lt_checks<const N: usize>(
        common_lhs: usize,
        rhss: [usize; N],
    ) -> Option<()> {
        const_for!(i in {0..N} {
            if common_lhs < rhss[i] {
                return None
            }
        });
        Some(())
    }

    #[doc(hidden)]
    #[inline]
    pub const fn unstable_common_ne_checks<const N: usize>(
        common_lhs: usize,
        rhss: [usize; N],
    ) -> Option<()> {
        const_for!(i in {0..N} {
            if common_lhs != rhss[i] {
                return None
            }
        });
        Some(())
    }

    /// this panics if `N == 0`
    #[doc(hidden)]
    #[inline]
    pub const fn unstable_max<const N: usize>(x: [usize; N]) -> usize {
        let mut max = x[0];
        const_for!(i in {1..N} {
            if x[i] > max {
                max = x[i];
            }
        });
        max
    }
}
