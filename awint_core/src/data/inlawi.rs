use core::{
    fmt,
    hash::{Hash, Hasher},
    mem,
    num::NonZeroUsize,
};

use awint_internals::*;

use crate::Bits;

/// Companion to `bits::_ASSERT_BITS_ASSUMPTIONS`
const _ASSERT_INLAWI_ASSUMPTIONS: () = {
    // 7 bits works on every platform
    let _ = ["Assertion that `InlAwi` size is what we expect"]
        [(mem::size_of::<InlAwi<7, 2>>() != (mem::size_of::<usize>() * 2)) as usize];
    let x = InlAwi::<7, 2>::imin();
    let _ = ["Assertion that layouts are working"]
        [((x.const_as_ref().raw_len() != 2) || (x.bw() != 7)) as usize];
    let _ = ["Assertion that values are working"][(x.const_as_ref().to_u8() != (1 << 6)) as usize];
};

// `InlAwi` has two parameters, because we absolutely have to have a parameter
// that directly specifies the raw array length, and because we also want Rust's
// typechecking to distinguish between different bitwidth `InlAwi`s.

/// An arbitrary width integer with const generic bitwidth that can be stored
/// inline on the stack like an array.
///
/// **NOTE**: Ideally, you could just type
/// `let _: InlAwi<100> = InlAwi<100>::zero();` if you wanted to specify and
/// construct an `InlAwi` type with bitwidth 100. However, Rust's lack of custom
/// DST support and const generics limitations makes this currently impossible.
/// The two const generic parameters of an `InlAwi` are part of a workaround for
/// this. Typing out a
/// `let _: InlAwi<BW, LEN> = InlAwi<BW, LEN>::zero()` should not be
/// done directly because it is non-portable and relies on unstable internal
/// details. Instead, you should use
///
/// `let _: inlawi_ty!(100) = inlawi_zero!(100);` or `let _ =
/// <inlawi_ty!(100)>::zero();` using macros from the `awint_macros` crate.
///
/// See the crate level documentation of `awint_macros` for more macros and
/// information.
#[derive(Clone, Copy)] // following what arrays do
pub struct InlAwi<const BW: usize, const LEN: usize> {
    /// # Raw Invariants
    ///
    /// The digits contained here have the raw invariants of `Bits`. This
    /// implies that `BW >= 2`, or else there is not enough storage for the
    /// first digit and metadata. The bitwidth must be set to value in the
    /// range `(((BW - 2)*BITS) + 1)..=((BW - 1)*BITS)`.
    raw: [usize; LEN],
}

/// `InlAwi` is safe to send between threads since it does not own
/// aliasing memory and has no reference counting mechanism like `Rc`.
unsafe impl<const BW: usize, const LEN: usize> Send for InlAwi<BW, LEN> {}

/// `InlAwi` is safe to share between threads since it does not own
/// aliasing memory and has no mutable internal state like `Cell` or `RefCell`.
unsafe impl<const BW: usize, const LEN: usize> Sync for InlAwi<BW, LEN> {}

impl<'a, const BW: usize, const LEN: usize> InlAwi<BW, LEN> {
    /// Returns a reference to `self` in the form of `&Bits`.
    #[inline]
    pub const fn const_as_ref(&'a self) -> &'a Bits {
        // Safety: Only functions like `unstable_from_slice` can construct the `raw`
        // field on `InlAwi`s. These always have the `assert_inlawi_invariants_` checks
        // to insure the raw invariants. The `_ASSERT_ASSUMPTIONS` constants make sure
        // this layout works. The explicit lifetimes make sure they do not
        // become unbounded.
        unsafe { Bits::from_raw_parts(self.raw.as_ptr(), self.raw.len()) }
    }

    /// Returns a reference to `self` in the form of `&mut Bits`.
    #[inline]
    pub const fn const_as_mut(&'a mut self) -> &'a mut Bits {
        // Safety: Only functions like `unstable_from_slice` can construct the `raw`
        // field on `InlAwi`s. These always have the `assert_inlawi_invariants_` checks
        // to insure the raw invariants. The `_ASSERT_ASSUMPTIONS` constants make sure
        // this layout works. The explicit lifetimes make sure they do not
        // become unbounded.
        unsafe { Bits::from_raw_parts_mut(self.raw.as_mut_ptr(), self.raw.len()) }
    }

    /// Returns the bitwidth of this `InlAwi` as a `NonZeroUsize`
    #[inline]
    pub const fn nzbw(&self) -> NonZeroUsize {
        self.const_as_ref().nzbw()
    }

    /// Returns the bitwidth of this `InlAwi` as a `usize`
    #[inline]
    pub const fn bw(&self) -> usize {
        self.const_as_ref().bw()
    }

    /// Returns the exact number of `usize` digits needed to store all bits.
    #[inline]
    pub const fn len(&self) -> usize {
        self.const_as_ref().len()
    }

    /// This is not intended for direct use, use `awint_macros::inlawi`
    /// or some other constructor instead.
    #[doc(hidden)]
    pub const fn unstable_from_slice(raw: &[usize]) -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        assert_inlawi_invariants_slice::<BW, LEN>(&raw);
        let mut copy = [0; LEN];
        const_for!(i in {0..raw.len()} {
            copy[i] = raw[i];
        });
        InlAwi { raw: copy }
    }

    /// Zero-value construction with bitwidth `BW`
    pub const fn zero() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        let mut raw = [0; LEN];
        raw[raw.len() - 1] = BW;
        assert_inlawi_invariants_slice::<BW, LEN>(&raw);
        InlAwi { raw }
    }

    /// Unsigned-maximum-value construction with bitwidth `BW`
    pub const fn umax() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        let mut raw = [MAX; LEN];
        raw[raw.len() - 1] = BW;
        assert_inlawi_invariants_slice::<BW, LEN>(&raw);
        let mut awi = InlAwi { raw };
        awi.const_as_mut().clear_unused_bits();
        awi
    }

    /// Signed-maximum-value construction with bitwidth `BW`
    pub const fn imax() -> Self {
        let mut awi = Self::umax();
        *awi.const_as_mut().last_mut() = (isize::MAX as usize) >> awi.const_as_ref().unused();
        awi
    }

    /// Signed-minimum-value construction with bitwidth `BW`
    pub const fn imin() -> Self {
        let mut awi = Self::zero();
        *awi.const_as_mut().last_mut() = (isize::MIN as usize) >> awi.const_as_ref().unused();
        awi
    }

    /// Unsigned-one-value construction with bitwidth `BW`
    pub const fn uone() -> Self {
        let mut awi = Self::zero();
        *awi.const_as_mut().first_mut() = 1;
        awi
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl<const BW: usize, const LEN: usize> PartialEq for InlAwi<BW, LEN> {
    fn eq(&self, rhs: &Self) -> bool {
        self.const_as_ref() == rhs.const_as_ref()
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl<const BW: usize, const LEN: usize> Eq for InlAwi<BW, LEN> {}

macro_rules! impl_fmt {
    ($($ty:ident)*) => {
        $(
            /// Forwards to the corresponding impl for `Bits`
            impl<const BW: usize, const LEN: usize> fmt::$ty for InlAwi<BW, LEN> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::$ty::fmt(self.const_as_ref(), f)
                }
            }
        )*
    };
}

impl_fmt!(Debug LowerHex UpperHex Octal Binary);

impl<const BW: usize, const LEN: usize> Hash for InlAwi<BW, LEN> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.const_as_ref().hash(state);
    }
}
