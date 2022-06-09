use awint_internals::*;

use crate::Bits;

// TODO we can avoid if statements by a clever enough single function, use
// wrapping arithmetic for the widths and cws, have a single inner FnOnce

/// Intended for use by the macros, for the purposes of avoiding `if` statements
/// with booleans for `awint_dag`, reducing the number of nested statements, and
/// avoiding requiring the user to import more than `Bit`s in some cases.
#[doc(hidden)]
impl Bits {
    /// Used by the macros to enforce compiler warnings
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
    #[inline]
    pub const fn unstable_raw_digits(bw: usize) -> usize {
        raw_digits(bw)
    }

    #[inline]
    pub const fn unstable_le_checks<const N: usize>(le_checks: [(usize, usize); N]) -> Option<()> {
        const_for!(i in {0..N} {
            if le_checks[i].0 > le_checks[i].1 {
                return None
            }
        });
        Some(())
    }

    #[inline]
    pub const fn unstable_common_checks<const N: usize, const M: usize>(
        common_cw: usize,
        ge: [usize; N],
        eq: [usize; M],
    ) -> Option<()> {
        const_for!(i in {0..N} {
            if common_cw < ge[i] {
                return None
            }
        });
        const_for!(i in {0..M} {
            if common_cw != eq[i] {
                return None
            }
        });
        Some(())
    }

    /// this panics if `N == 0`
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
