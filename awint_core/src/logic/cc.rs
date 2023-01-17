use core::marker::PhantomData;

use awint_internals::*;

use crate::Bits;

#[doc(hidden)]
pub struct CCResult<T> {
    run_fielding: bool,
    success: bool,
    _phantom_data: PhantomData<T>,
}

impl<T> CCResult<T> {
    #[inline]
    pub const fn run_fielding(&self) -> bool {
        self.run_fielding
    }

    #[inline]
    pub const fn wrap(self, t: T) -> Option<T> {
        Some(t)
    }

    #[inline]
    pub const fn wrap_none(self) -> Option<T> {
        None
    }
}

impl CCResult<()> {
    #[inline]
    pub const fn wrap_if_success(self) -> Option<()> {
        if self.success {
            Some(())
        } else {
            None
        }
    }
}

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

    /// Exists to fix type problems for `awint_dag`
    #[inline]
    pub const fn usize_cast(x: usize) -> usize {
        x
    }

    /// This exists because of problems with "can't call method `wrapping_add`
    /// on ambiguous numeric type `{integer}`" and how we can't have mixed
    /// `awint_dag` types if we specify types at the let binding.
    #[inline]
    pub const fn usize_add(lhs: usize, rhs: usize) -> usize {
        lhs.wrapping_add(rhs)
    }

    #[inline]
    pub const fn usize_sub(lhs: usize, rhs: usize) -> usize {
        lhs.wrapping_sub(rhs)
    }

    /// This is for the macros crate to plug into the `LEN` generic in
    /// `InlAwi<BW, LEN>`, because the build architecture pointer width can be
    /// different from the target architecture pointer width (and we can't use
    /// `target_pointer_width` because it corresponds to whatever architecture
    /// the procedural macro crate is running for).
    #[inline]
    pub const fn unstable_raw_digits(w: usize) -> usize {
        raw_digits(w)
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

    #[inline]
    pub const fn unstable_cc_checks<const LE: usize, const GE: usize, const EQ: usize, T>(
        le0: [usize; LE],
        le1: [usize; LE],
        ge: [usize; GE],
        eq: [usize; EQ],
        cw: usize,
        check_nonzero_cw: bool,
        ok_on_zero: bool,
    ) -> CCResult<T> {
        let none = CCResult {
            run_fielding: false,
            success: false,
            _phantom_data: PhantomData,
        };
        const_for!(i in {0..LE} {
            if le0[i] > le1[i] {
                return none
            }
        });
        const_for!(i in {0..GE} {
            if cw < ge[i] {
                return none
            }
        });
        const_for!(i in {0..EQ} {
            if cw != eq[i] {
                return none
            }
        });
        if check_nonzero_cw && (cw == 0) {
            if ok_on_zero {
                return CCResult {
                    run_fielding: false,
                    success: true,
                    _phantom_data: PhantomData,
                }
            } else {
                return none
            }
        }
        CCResult {
            run_fielding: true,
            success: true,
            _phantom_data: PhantomData,
        }
    }
}
