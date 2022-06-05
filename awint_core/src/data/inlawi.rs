use core::{
    fmt,
    hash::{Hash, Hasher},
    num::NonZeroUsize,
};

use awint_internals::*;
use const_fn::const_fn;

use crate::Bits;

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
///
/// ```
/// #![feature(const_trait_impl)]
/// #![feature(const_mut_refs)]
/// #![feature(const_option)]
/// use awint::{cc, inlawi, inlawi_ty, Bits, InlAwi};
///
/// const fn const_fn(mut lhs: &mut Bits, rhs: &Bits) {
///     // `InlAwi` stored on the stack does no allocation
///     let mut tmp = inlawi!(0i100);
///     tmp.mul_add_assign(lhs, rhs).unwrap();
///     cc!(tmp; lhs).unwrap();
/// }
///
/// // Because `InlAwi`'s construction functions are `const`, we can make full
/// // use of `Bits` `const` abilities
/// const AWI: inlawi_ty!(100) = {
///     let mut awi0 = inlawi!(123i100);
///     let x = awi0.const_as_mut();
///     let awi1 = inlawi!(2i100);
///     let y = awi1.const_as_ref();
///     x.neg_assign(true);
///     const_fn(x, y);
///     awi0
/// };
/// const X: &'static Bits = AWI.const_as_ref();
///
/// assert_eq!(X, inlawi!(-246i100).const_as_ref());
/// ```
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
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn const_as_ref(&'a self) -> &'a Bits {
        // Safety: Only functions like `unstable_from_u8_slice` can construct the `raw`
        // field on `InlAwi`s. These always have the `assert_inlawi_invariants_` checks
        // to insure the raw invariants. The explicit lifetimes make sure they do not
        // become unbounded.
        unsafe { Bits::from_raw_parts(self.raw.as_ptr(), self.raw.len()) }
    }

    /// Returns a reference to `self` in the form of `&mut Bits`.
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn const_as_mut(&'a mut self) -> &'a mut Bits {
        // Safety: Only functions like `unstable_from_u8_slice` can construct the `raw`
        // field on `InlAwi`s. These always have the `assert_inlawi_invariants_` checks
        // to insure the raw invariants. The explicit lifetimes make sure they do not
        // become unbounded.
        unsafe { Bits::from_raw_parts_mut(self.raw.as_mut_ptr(), self.raw.len()) }
    }

    /// Returns the bitwidth of this type of `InlAwi` as a `NonZeroUsize`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn const_nzbw() -> NonZeroUsize {
        assert_inlawi_invariants::<BW, LEN>();
        NonZeroUsize::new(BW).unwrap()
    }

    /// Returns the bitwidth of this type of `InlAwi` as a `usize`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn const_bw() -> usize {
        assert_inlawi_invariants::<BW, LEN>();
        BW
    }

    /// The same as `Self::const_nzbw()` except that it takes `&self`, this
    /// exists to help with macros
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn nzbw(&self) -> NonZeroUsize {
        Self::const_nzbw()
    }

    /// The same as `Self::const_bw()` except that it takes `&self`, this exists
    /// to help with macros
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn bw(&self) -> usize {
        Self::const_bw()
    }

    /// Returns the exact number of `usize` digits needed to store all bits.
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn len(&self) -> usize {
        self.const_as_ref().len()
    }

    /// This is not intended for direct use, use `awint_macros::inlawi`
    /// or some other constructor instead. The purpose of this function is to
    /// allow for a `usize::BITS` difference between a target architecture and
    /// the build architecture. Uses `u8_slice_assign`.
    #[doc(hidden)]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn unstable_from_u8_slice(buf: &[u8]) -> Self {
        if LEN < 2 {
            panic!("Tried to create an `InlAwi<BW, LEN>` with `LEN < 2`")
        }
        let mut raw = [0; LEN];
        raw[raw.len() - 1] = BW;
        assert_inlawi_invariants_slice::<BW, LEN>(&raw);
        let mut awi = InlAwi { raw };
        awi.const_as_mut().u8_slice_assign(buf);
        awi
    }

    /// Zero-value construction with bitwidth `BW`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn zero() -> Self {
        if LEN < 2 {
            panic!("Tried to create an `InlAwi<BW, LEN>` with `LEN < 2`")
        }
        let mut raw = [0; LEN];
        raw[raw.len() - 1] = BW;
        assert_inlawi_invariants_slice::<BW, LEN>(&raw);
        InlAwi { raw }
    }

    /// Unsigned-maximum-value construction with bitwidth `BW`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn umax() -> Self {
        if LEN < 2 {
            panic!("Tried to create an `InlAwi<BW, LEN>` with `LEN < 2`")
        }
        let mut raw = [MAX; LEN];
        raw[raw.len() - 1] = BW;
        assert_inlawi_invariants_slice::<BW, LEN>(&raw);
        let mut awi = InlAwi { raw };
        awi.const_as_mut().clear_unused_bits();
        awi
    }

    /// Signed-maximum-value construction with bitwidth `BW`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn imax() -> Self {
        let mut awi = Self::umax();
        *awi.const_as_mut().last_mut() = (isize::MAX as usize) >> awi.const_as_ref().unused();
        awi
    }

    /// Signed-minimum-value construction with bitwidth `BW`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn imin() -> Self {
        let mut awi = Self::zero();
        *awi.const_as_mut().last_mut() = (isize::MIN as usize) >> awi.const_as_ref().unused();
        awi
    }

    /// Unsigned-one-value construction with bitwidth `BW`
    #[const_fn(cfg(feature = "const_support"))]
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

impl_fmt!(Debug Display LowerHex UpperHex Octal Binary);

impl<const BW: usize, const LEN: usize> Hash for InlAwi<BW, LEN> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.const_as_ref().hash(state);
    }
}

// FIXME
// maybe use const `From` in addition to this?
/*impl InlAwi<8, 2> {
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn from_u8(x: u8) -> Self {
        let mut awi = Self::zero();
        awi.const_as_mut().u8_assign(x);
        awi
    }
}

impl InlAwi<64, 2> {
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn from_u64(x: u64) -> Self {
        let mut awi = Self::zero();
        awi.const_as_mut().u64_assign(x);
        awi
    }
}*/
