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
/// `let _: inlawi_ty!(100) = inlawi!(0u100);` or `let _ =
/// <inlawi_ty!(100)>::zero();` using macros from the `awint_macros` crate.
///
/// See the crate level documentation of `awint_macros` for more macros and
/// information.
///
/// ```
/// // only needed if you are trying to use in `const` contexts
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
///     let mut x = inlawi!(123i100);
///     let y = inlawi!(2i100);
///     x.neg_assign(true);
///     const_fn(&mut x, &y);
///     x
/// };
/// const X: &'static Bits = &AWI;
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
    #[must_use]
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
    #[must_use]
    pub const fn const_as_mut(&'a mut self) -> &'a mut Bits {
        // Safety: Only functions like `unstable_from_u8_slice` can construct the `raw`
        // field on `InlAwi`s. These always have the `assert_inlawi_invariants_` checks
        // to insure the raw invariants. The explicit lifetimes make sure they do not
        // become unbounded.
        unsafe { Bits::from_raw_parts_mut(self.raw.as_mut_ptr(), self.raw.len()) }
    }

    /// Returns the bitwidth of this type of `InlAwi` as a `NonZeroUsize`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn const_nzbw() -> NonZeroUsize {
        assert_inlawi_invariants::<BW, LEN>();
        NonZeroUsize::new(BW).unwrap()
    }

    /// Returns the bitwidth of this type of `InlAwi` as a `usize`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn const_bw() -> usize {
        assert_inlawi_invariants::<BW, LEN>();
        BW
    }

    /// Returns the raw length of this type of `InlAwi` as a `usize`
    #[doc(hidden)]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn const_raw_len() -> usize {
        assert_inlawi_invariants::<BW, LEN>();
        LEN
    }

    /// The same as `Self::const_nzbw()` except that it takes `&self`, this
    /// exists to help with macros
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn nzbw(&self) -> NonZeroUsize {
        Self::const_nzbw()
    }

    /// The same as `Self::const_bw()` except that it takes `&self`, this exists
    /// to help with macros
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn bw(&self) -> usize {
        Self::const_bw()
    }

    /// Returns the exact number of `usize` digits needed to store all bits.
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn len(&self) -> usize {
        Self::const_raw_len() - 1
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
            impl<const BW: usize, const LEN: usize> fmt::$ty for InlAwi<BW, LEN> {
                /// Forwards to the corresponding impl for `Bits`
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

impl InlAwi<1, { Bits::unstable_raw_digits(1) }> {
    /// Creates an `InlAwi` with one bit set to this `bool`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn from_bool(x: bool) -> Self {
        let mut awi = Self::zero();
        awi.const_as_mut().bool_assign(x);
        awi
    }
}

macro_rules! inlawi_from {
    ($($w:expr, $u:ident $from_u:ident $u_assign:ident
        $i:ident $from_i:ident $i_assign:ident);*;) => {
        $(
            impl InlAwi<$w, {Bits::unstable_raw_digits($w)}> {
                /// Creates an `InlAwi` with the same bitwidth and bits as the integer
                #[const_fn(cfg(feature = "const_support"))]
                pub const fn $from_u(x: $u) -> Self {
                    let mut awi = Self::zero();
                    awi.const_as_mut().$u_assign(x);
                    awi
                }

                /// Creates an `InlAwi` with the same bitwidth and bits as the integer
                #[const_fn(cfg(feature = "const_support"))]
                pub const fn $from_i(x: $i) -> Self {
                    let mut awi = Self::zero();
                    awi.const_as_mut().$i_assign(x);
                    awi
                }
            }
        )*
    };
}

inlawi_from!(
    8, u8 from_u8 u8_assign i8 from_i8 i8_assign;
    16, u16 from_u16 u16_assign i16 from_i16 i16_assign;
    32, u32 from_u32 u32_assign i32 from_i32 i32_assign;
    64, u64 from_u64 u64_assign i64 from_i64 i64_assign;
    128, u128 from_u128 u128_assign i128 from_i128 i128_assign;
);

pub(crate) type UsizeInlAwi =
    InlAwi<{ usize::BITS as usize }, { Bits::unstable_raw_digits(usize::BITS as usize) }>;

impl UsizeInlAwi {
    /// Creates an `InlAwi` with the same bitwidth and bits as the integer
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn from_usize(x: usize) -> Self {
        let mut awi = Self::zero();
        awi.const_as_mut().usize_assign(x);
        awi
    }

    /// Creates an `InlAwi` with the same bitwidth and bits as the integer
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn from_isize(x: isize) -> Self {
        let mut awi = Self::zero();
        awi.const_as_mut().isize_assign(x);
        awi
    }
}
