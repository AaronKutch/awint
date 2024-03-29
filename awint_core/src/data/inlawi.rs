use core::{
    fmt,
    hash::{Hash, Hasher},
    num::NonZeroUsize,
    ptr,
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
/// This struct implements `Deref<Target = Bits>`, see the main documentation of
/// [Bits](crate::Bits) for more. There are also some allocating functions that
/// only `ExtAwi` and `Awi` implement.
///
/// ```
/// use awint::{cc, inlawi, inlawi_ty, Bits, InlAwi};
///
/// fn example(mut lhs: &mut Bits, rhs: &Bits) {
///     // `InlAwi` stored on the stack does no allocation
///     let mut tmp = inlawi!(0i100);
///     tmp.mul_add_(lhs, rhs).unwrap();
///     cc!(tmp; lhs).unwrap();
/// }
///
/// let val: inlawi_ty!(100) = {
///     let mut x = inlawi!(123i100);
///     let y = inlawi!(2i100);
///     x.neg_(true);
///     example(&mut x, &y);
///     x
/// };
/// let x: &Bits = &val;
///
/// assert_eq!(x, inlawi!(-246i100).as_ref());
/// ```
// FIXME
/// ```text
/// // note: see README because this is broken on some nightlies
///
/// // only needed if you are trying to use in `const` contexts
/// #![feature(const_trait_impl)]
/// #![feature(const_mut_refs)]
/// #![feature(const_option)]
/// use awint::{cc, inlawi, inlawi_ty, Bits, InlAwi};
///
/// const fn const_example(mut lhs: &mut Bits, rhs: &Bits) {
///     // `InlAwi` stored on the stack does no allocation
///     let mut tmp = inlawi!(0i100);
///     tmp.mul_add_(lhs, rhs).unwrap();
///     cc!(tmp; lhs).unwrap();
/// }
///
/// // Because `InlAwi`'s construction functions are `const`, we can make full
/// // use of `Bits` `const` abilities
/// const AWI: inlawi_ty!(100) = {
///     let mut x = inlawi!(123i100);
///     let y = inlawi!(2i100);
///     x.neg_(true);
///     const_example(&mut x, &y);
///     x
/// };
/// const X: &'static Bits = &AWI;
///
/// assert_eq!(X, inlawi!(-246i100).as_ref());
/// ```
#[repr(C)]
#[derive(Clone, Copy)] // following what arrays do
pub struct InlAwi<const BW: usize, const LEN: usize> {
    _raw_stack_bits: RawStackBits<BW, LEN>,
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
    pub(in crate::data) const fn internal_as_ref(&'a self) -> &'a Bits {
        // Safety: Only functions like `unstable_from_u8_slice` can construct the
        // `_raw_stack_bits` field on `InlAwi`s. The `RawStackBits` functions
        // checks to insure the raw invariants. The explicit lifetimes make sure they do
        // not become unbounded.
        unsafe { Bits::from_raw_parts(self._raw_stack_bits.to_raw_bits()) }
    }

    /// Returns a reference to `self` in the form of `&mut Bits`.
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub(in crate::data) const fn internal_as_mut(&'a mut self) -> &'a mut Bits {
        // Safety: Same as `internal_as_ref`, except we use `to_raw_bits_mut` so that
        // the pointer tag is not `Frozen`
        unsafe { Bits::from_raw_parts_mut(self._raw_stack_bits.to_raw_bits_mut()) }
    }

    /// Returns the bitwidth of this type of `InlAwi` as a `NonZeroUsize`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn const_nzbw() -> NonZeroUsize {
        RawStackBits::<BW, LEN>::nzbw()
    }

    /// Returns the bitwidth of this type of `InlAwi` as a `usize`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn const_bw() -> usize {
        RawStackBits::<BW, LEN>::bw()
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

    /// Returns the exact number of `Digit`s needed to store all bits.
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn total_digits(&self) -> usize {
        // TODO use `NonZeroUsize` in the `len` and `total_digits` returns?
        RawStackBits::<BW, LEN>::total_digits().get()
    }

    /// This is not intended for direct use, use `awint_macros::inlawi`
    /// or some other constructor instead. The purpose of this function is to
    /// allow for a `Digit::BITS` difference between a target architecture and
    /// the build architecture. Uses `u8_slice_`.
    #[doc(hidden)]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn unstable_from_u8_slice(buf: &[u8]) -> Self {
        let mut val = InlAwi {
            _raw_stack_bits: RawStackBits::zero(),
        };

        // At this point, we would call `val.u8_slice_(buf)` and could return that.
        // However, we run into a problem where the compiler has difficulty. For
        // example,
        //
        // ```
        // pub fn internal(x: &mut Bits, neg: bool) {
        //     let y = inlawi!(-3858683298722959599393.23239873423987_i1024f512);
        //     x.neg_add_(neg, &y).unwrap();
        // }
        // ```
        //
        // should compile into a function that reserves 128 bytes (plus 32 or so more
        // depending on architecture) on the stack pointer, a single
        // memcpy, and then a call to `neg_add_`. However, if we don't inline
        // most functions it will get confused, reserve a further 128 bytes, add
        // a memcpy, and a memset. This is bad in general but very bad for architectures
        // like AVR, where we don't want to double stack reservations.

        // inline `u8_slice_`, but we can also eliminate the `digit_set`
        let self_byte_width = val.total_digits() * DIGIT_BYTES;
        let min_width = if self_byte_width < buf.len() {
            self_byte_width
        } else {
            buf.len()
        };
        // start of digits that will not be completely overwritten
        let start = min_width / DIGIT_BYTES;
        unsafe {
            // zero out first.
            // (skip because we initialized `raw` to all zeros)
            //val.digit_set(false, start..self.total_digits(), false);
            // Safety: `src` is valid for reads at least up to `min_width`, `dst` is valid
            // for writes at least up to `min_width`, they are aligned, and are
            // nonoverlapping because `self` is a mutable reference.
            ptr::copy_nonoverlapping(
                buf.as_ptr(),
                // note that we marked this as `#[inline]`
                val.as_mut_bytes_full_width_nonportable().as_mut_ptr(),
                min_width,
            );
            // `start` can be `self.total_digits()`, so cap it
            let cap = if start >= val.total_digits() {
                val.total_digits()
            } else {
                start + 1
            };
            const_for!(i in {0..cap} {
                // correct for big endian, otherwise no-op
                *val.get_unchecked_mut(i) = Digit::from_le(val.get_unchecked(i));
            });
        }
        val.clear_unused_bits();

        val
    }

    /// Zero-value construction with bitwidth `BW`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn zero() -> Self {
        Self {
            _raw_stack_bits: RawStackBits::zero(),
        }
    }

    /// Unsigned-maximum-value construction with bitwidth `BW`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn umax() -> Self {
        let mut val = Self {
            _raw_stack_bits: RawStackBits::all_set(),
        };
        val.const_as_mut().clear_unused_bits();
        val
    }

    /// Signed-maximum-value construction with bitwidth `BW`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn imax() -> Self {
        let mut val = Self::umax();
        *val.const_as_mut().last_mut() = (MAX >> 1) >> val.unused();
        val
    }

    /// Signed-minimum-value construction with bitwidth `BW`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn imin() -> Self {
        let mut val = Self::zero();
        *val.const_as_mut().last_mut() = (IDigit::MIN as Digit) >> val.unused();
        val
    }

    /// Unsigned-one-value construction with bitwidth `BW`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn uone() -> Self {
        let mut val = Self::zero();
        *val.const_as_mut().first_mut() = 1;
        val
    }

    /// Used for tests, Can't put in `awint_internals`
    #[doc(hidden)]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn assert_invariants(this: &Self) {
        // double check
        RawStackBits::<BW, LEN>::_assert_invariants();
        // not a strict invariant but we want to test it
        this.assert_cleared_unused_bits();
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl<const BW: usize, const LEN: usize> PartialEq for InlAwi<BW, LEN> {
    fn eq(&self, rhs: &Self) -> bool {
        self.as_ref() == rhs.as_ref()
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl<const BW: usize, const LEN: usize> Eq for InlAwi<BW, LEN> {}

#[cfg(feature = "zeroize_support")]
impl<const BW: usize, const LEN: usize> zeroize::Zeroize for InlAwi<BW, LEN> {
    fn zeroize(&mut self) {
        self.as_mut().zeroize()
    }
}

macro_rules! impl_fmt {
    ($($ty:ident)*) => {
        $(
            impl<const BW: usize, const LEN: usize> fmt::$ty for InlAwi<BW, LEN> {
                /// Forwards to the corresponding impl for `Bits`
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::$ty::fmt(self.as_ref(), f)
                }
            }
        )*
    };
}

impl_fmt!(Debug Display LowerHex UpperHex Octal Binary);

impl<const BW: usize, const LEN: usize> Hash for InlAwi<BW, LEN> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl InlAwi<1, { Bits::unstable_raw_digits(1) }> {
    /// Creates an `InlAwi` with one bit set to this `bool`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn from_bool(x: bool) -> Self {
        let mut val = Self::zero();
        val.bool_(x);
        val
    }
}

macro_rules! inlawi_from {
    ($($w:expr, $u:ident $from_u:ident $u_:ident
        $i:ident $from_i:ident $i_:ident);*;) => {
        $(
            impl InlAwi<$w, {Bits::unstable_raw_digits($w)}> {
                /// Creates an `InlAwi` with the same bitwidth and bits as the integer
                #[const_fn(cfg(feature = "const_support"))]
                pub const fn $from_u(x: $u) -> Self {
                    let mut val = Self::zero();
                    val.$u_(x);
                    val
                }

                /// Creates an `InlAwi` with the same bitwidth and bits as the integer
                #[const_fn(cfg(feature = "const_support"))]
                pub const fn $from_i(x: $i) -> Self {
                    let mut val = Self::zero();
                    val.$i_(x);
                    val
                }
            }
        )*
    };
}

inlawi_from!(
    8, u8 from_u8 u8_ i8 from_i8 i8_;
    16, u16 from_u16 u16_ i16 from_i16 i16_;
    32, u32 from_u32 u32_ i32 from_i32 i32_;
    64, u64 from_u64 u64_ i64 from_i64 i64_;
    128, u128 from_u128 u128_ i128 from_i128 i128_;
);

pub(crate) type UsizeInlAwi = InlAwi<{ USIZE_BITS }, { Bits::unstable_raw_digits(USIZE_BITS) }>;

impl UsizeInlAwi {
    /// Creates an `InlAwi` with the same bitwidth and bits as the integer
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn from_usize(x: usize) -> Self {
        let mut val = Self::zero();
        val.usize_(x);
        val
    }

    /// Creates an `InlAwi` with the same bitwidth and bits as the integer
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn from_isize(x: isize) -> Self {
        let mut val = Self::zero();
        val.isize_(x);
        val
    }
}

pub(crate) type DigitInlAwi = InlAwi<{ BITS }, { Bits::unstable_raw_digits(BITS) }>;

impl DigitInlAwi {
    /// Creates an `InlAwi` with the same bitwidth and bits as `Digit`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn from_digit(x: Digit) -> Self {
        let mut val = Self::zero();
        val.digit_(x);
        val
    }
}
