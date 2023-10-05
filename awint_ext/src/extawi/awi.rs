use alloc::alloc::{alloc, alloc_zeroed, dealloc, Layout};
use core::{
    borrow::{Borrow, BorrowMut},
    fmt,
    hash::{Hash, Hasher},
    mem,
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Index, IndexMut, RangeFull},
    ptr,
    ptr::NonNull,
};

use awint_core::{Bits, InlAwi};
use const_fn::const_fn;

use crate::awint_internals::*;

#[inline]
pub(crate) const fn layout(w: NonZeroUsize) -> Layout {
    // Safety: this produces the exact number of bytes needed to satisfy the raw
    // invariants of `Bits`.
    unsafe {
        Layout::from_size_align_unchecked(
            total_digits(w).get() * mem::size_of::<Digit>(),
            mem::align_of::<Digit>(),
        )
    }
}

/// An arbitrary width integer with manually controlled bitwidth. Most
/// arithmetic is wrapping like Rust's integers. All reallocations are
/// explicit.
///
/// This struct implements `Deref<Target = Bits>`, see the main documentation of
/// [Bits](awint_core::Bits) for more. There are also some functions that
/// `InlAwi` and `Bits` do not implement, namely some higher level string
/// serialization functions that require allocation. Also note that `ExtAwi` and
/// `Awi` cannot take advantage of the `const`ness of `Bits` operations, see
/// [InlAwi](awint_core::InlAwi).
///
/// See the crate level documentation of `awint_macros` for more macros and
/// information.
///
/// ```
/// use awint::awi::*;
///
/// fn example(x0: &mut Bits, x1: &Bits) {
///     // when dealing with `Bits` with different bitwidths, use the
///     // `*_resize_` functions or the concatenations of components
///     // macros with unbounded fillers from `awint_macros`
///     x0.sign_resize_(x1);
///     // multiply in place by 2 for an example
///     x0.digit_cin_mul_(0, 2);
/// }
///
/// // using `bw` function for quick `NonZeroUsize` construction from a literal
/// let mut x = ExtAwi::zero(bw(100));
/// assert!(x.is_zero());
/// // constructing an `ExtAwi` from an `InlAwi`
/// let y = ExtAwi::from(inlawi!(-123i16));
/// example(&mut x, &y);
/// assert_eq!(x.as_ref(), inlawi!(-246i100).as_ref());
/// // you can freely mix references originating from both `ExtAwi` and `InlAwi`
/// example(&mut x, &inlawi!(0x10u16));
/// assert_eq!(x, extawi!(0x20u100));
/// ```
#[repr(C)]
pub struct ExtAwi {
    _raw_bits: RawBits,
}

/// `ExtAwi` is safe to send between threads since it does not own
/// aliasing memory and has no reference counting mechanism like `Rc`.
unsafe impl Send for ExtAwi {}

/// `ExtAwi` is safe to share between threads since it does not own
/// aliasing memory and has no mutable internal state like `Cell` or `RefCell`.
unsafe impl Sync for ExtAwi {}

impl<'a> ExtAwi {
    /// # Safety
    ///
    /// `ptr` should be allocated according to the `extawi::layout` function and
    /// all digits initialized. The `Bits` raw invariants should be followed.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn from_raw_parts(digits: NonNull<Digit>, bw: NonZeroUsize) -> ExtAwi {
        ExtAwi {
            _raw_bits: RawBits::from_raw_parts(digits, bw),
        }
    }

    /// Returns a reference to `self` in the form of `&Bits`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    const fn internal_as_ref(&'a self) -> &'a Bits {
        // Safety: We copy the `RawBits`, and the explicit lifetimes make sure
        // they do not become unbounded.
        unsafe {
            Bits::from_raw_parts(RawBits::from_raw_parts(
                self._raw_bits.as_non_null_ptr(),
                self._raw_bits.nzbw(),
            ))
        }
    }

    /// Returns a reference to `self` in the form of `&mut Bits`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    const fn internal_as_mut(&'a mut self) -> &'a mut Bits {
        // Safety: We copy the `RawBits`, and the explicit lifetimes make sure
        // they do not become unbounded.
        unsafe {
            Bits::from_raw_parts_mut(RawBits::from_raw_parts(
                self._raw_bits.as_non_null_ptr(),
                self._raw_bits.nzbw(),
            ))
        }
    }

    /// Returns the bitwidth of this `ExtAwi` as a `NonZeroUsize`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn nzbw(&self) -> NonZeroUsize {
        self._raw_bits.nzbw()
    }

    /// Returns the bitwidth of this `ExtAwi` as a `usize`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn bw(&self) -> usize {
        self._raw_bits.bw()
    }

    /// Returns the total number of digits of this `ExtAwi`
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn total_digits(&self) -> NonZeroUsize {
        self._raw_bits.total_digits()
    }

    /// Returns a raw pointer to the underlying bit storage. The caller must
    /// ensure that the `ExtAwi` outlives the pointer this function returns,
    /// and that the `ExtAwi` is not reallocated. The underlying memory should
    /// never be written to.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub fn as_ptr(&self) -> *const Digit {
        self.internal_as_ref().as_ptr()
    }

    /// Returns a raw pointer to the underlying bit storage. The caller must
    /// ensure that the `ExtAwi` outlives the pointer this function returns,
    /// and that the `ExtAwi` is not reallocated.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut Digit {
        self.internal_as_mut().as_mut_ptr()
    }

    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn layout(&self) -> Layout {
        layout(self.nzbw())
    }

    /// Creates an `ExtAwi` from copying a `Bits` reference. The same
    /// functionality is provided by an `From<&Bits>` implementation for
    /// `ExtAwi`.
    pub fn from_bits(bits: &Bits) -> ExtAwi {
        let mut tmp = ExtAwi::zero(bits.nzbw());
        tmp.const_as_mut().copy_(bits).unwrap();
        tmp
    }

    /// Zero-value construction with bitwidth `w`
    pub fn zero(w: NonZeroUsize) -> Self {
        // Safety: This satisfies `ExtAwi::from_raw_parts`
        unsafe {
            let ptr: *mut Digit = alloc_zeroed(layout(w)).cast();
            ExtAwi::from_raw_parts(NonNull::new_unchecked(ptr), w)
        }
    }

    /// Unsigned-maximum-value construction with bitwidth `w`
    pub fn umax(w: NonZeroUsize) -> Self {
        // Safety: This satisfies `ExtAwi::from_raw_parts`
        let mut x = unsafe {
            let ptr: *mut Digit = alloc(layout(w)).cast();
            ptr.write_bytes(u8::MAX, total_digits(w).get());
            ExtAwi::from_raw_parts(NonNull::new_unchecked(ptr), w)
        };
        x.const_as_mut().clear_unused_bits();
        x
    }

    /// Signed-maximum-value construction with bitwidth `w`
    pub fn imax(w: NonZeroUsize) -> Self {
        let mut val = Self::umax(w);
        *val.const_as_mut().last_mut() = (MAX >> 1) >> val.unused();
        val
    }

    /// Signed-minimum-value construction with bitwidth `w`
    pub fn imin(w: NonZeroUsize) -> Self {
        let mut val = Self::zero(w);
        *val.const_as_mut().last_mut() = (IDigit::MIN as Digit) >> val.unused();
        val
    }

    /// Unsigned-one-value construction with bitwidth `w`
    pub fn uone(w: NonZeroUsize) -> Self {
        let mut val = Self::zero(w);
        *val.const_as_mut().first_mut() = 1;
        val
    }

    /// Used by `awint_macros` in avoiding a `NonZeroUsize` dependency
    #[doc(hidden)]
    pub fn panicking_zero(w: usize) -> Self {
        Self::zero(NonZeroUsize::new(w).unwrap())
    }

    /// Used by `awint_macros` in avoiding a `NonZeroUsize` dependency
    #[doc(hidden)]
    pub fn panicking_umax(w: usize) -> Self {
        Self::umax(NonZeroUsize::new(w).unwrap())
    }

    /// Used by `awint_macros` in avoiding a `NonZeroUsize` dependency
    #[doc(hidden)]
    pub fn panicking_imax(w: usize) -> Self {
        Self::imax(NonZeroUsize::new(w).unwrap())
    }

    /// Used by `awint_macros` in avoiding a `NonZeroUsize` dependency
    #[doc(hidden)]
    pub fn panicking_imin(w: usize) -> Self {
        Self::imin(NonZeroUsize::new(w).unwrap())
    }

    /// Used by `awint_macros` in avoiding a `NonZeroUsize` dependency
    #[doc(hidden)]
    pub fn panicking_uone(w: usize) -> Self {
        Self::uone(NonZeroUsize::new(w).unwrap())
    }
}

impl Drop for ExtAwi {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.as_mut_ptr().cast(), self.layout());
        }
    }
}

impl Clone for ExtAwi {
    fn clone(&self) -> ExtAwi {
        // Safety: The allocation is made according to the layout, digits are copied
        // according to digit length, and the `ExtAwi` is constructed with bitwidth
        unsafe {
            let dst = alloc(self.layout()).cast();
            ptr::copy_nonoverlapping(self.as_ptr(), dst, self.total_digits().get());
            ExtAwi::from_raw_parts(NonNull::new_unchecked(dst), self.nzbw())
        }
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl PartialEq for ExtAwi {
    fn eq(&self, rhs: &Self) -> bool {
        self.as_ref() == rhs.as_ref()
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl Eq for ExtAwi {}

#[cfg(feature = "zeroize_support")]
impl zeroize::Zeroize for ExtAwi {
    fn zeroize(&mut self) {
        self.as_mut().zeroize()
    }
}

macro_rules! impl_fmt {
    ($($ty:ident)*) => {
        $(
            /// Forwards to the corresponding impl for `Bits`
            impl fmt::$ty for ExtAwi {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::$ty::fmt(self.as_ref(), f)
                }
            }
        )*
    };
}

impl_fmt!(Debug Display LowerHex UpperHex Octal Binary);

impl Hash for ExtAwi {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl Deref for ExtAwi {
    type Target = Bits;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.internal_as_ref()
    }
}

impl DerefMut for ExtAwi {
    #[inline]
    fn deref_mut(&mut self) -> &mut Bits {
        self.internal_as_mut()
    }
}

impl Index<RangeFull> for ExtAwi {
    type Output = Bits;

    #[inline]
    fn index(&self, _i: RangeFull) -> &Bits {
        self
    }
}

impl Borrow<Bits> for ExtAwi {
    #[inline]
    fn borrow(&self) -> &Bits {
        self
    }
}

impl AsRef<Bits> for ExtAwi {
    #[inline]
    fn as_ref(&self) -> &Bits {
        self
    }
}

impl IndexMut<RangeFull> for ExtAwi {
    #[inline]
    fn index_mut(&mut self, _i: RangeFull) -> &mut Bits {
        self
    }
}

impl BorrowMut<Bits> for ExtAwi {
    #[inline]
    fn borrow_mut(&mut self) -> &mut Bits {
        self
    }
}

impl AsMut<Bits> for ExtAwi {
    #[inline]
    fn as_mut(&mut self) -> &mut Bits {
        self
    }
}

// we unfortunately can't do something like `impl<B: Borrow<Bits>> From<B>`
// because specialization is not stabilized

/// Creates an `ExtAwi` from copying a `Bits` reference
impl From<&Bits> for ExtAwi {
    fn from(bits: &Bits) -> ExtAwi {
        let mut tmp = ExtAwi::zero(bits.nzbw());
        tmp.const_as_mut().copy_(bits).unwrap();
        tmp
    }
}

/// Creates an `ExtAwi` from copying an `InlAwi`
impl<const BW: usize, const LEN: usize> From<InlAwi<BW, LEN>> for ExtAwi {
    fn from(awi: InlAwi<BW, LEN>) -> ExtAwi {
        let mut tmp = ExtAwi::zero(awi.nzbw());
        tmp.const_as_mut().copy_(&awi).unwrap();
        tmp
    }
}

macro_rules! extawi_from_ty {
    ($($ty:ident $from:ident $assign:ident);*;) => {
        $(
            /// Creates an `ExtAwi` with the same bitwidth and bits as the integer
            pub fn $from(x: $ty) -> Self {
                let mut tmp = ExtAwi::zero(bw($ty::BITS as usize));
                tmp.$assign(x);
                tmp
            }
        )*
    };
}

impl ExtAwi {
    extawi_from_ty!(
        u8 from_u8 u8_;
        u16 from_u16 u16_;
        u32 from_u32 u32_;
        u64 from_u64 u64_;
        u128 from_u128 u128_;
        usize from_usize usize_;
        i8 from_i8 i8_;
        i16 from_i16 i16_;
        i32 from_i32 i32_;
        i64 from_i64 i64_;
        i128 from_i128 i128_;
        isize from_isize isize_;
    );

    /// Creates an `ExtAwi` with one bit set to this `bool`
    pub fn from_bool(x: bool) -> Self {
        let mut tmp = ExtAwi::zero(bw(1));
        tmp.bool_(x);
        tmp
    }

    /// Creates an `ExtAwi` with the same bitwidth and bits as the integer
    pub fn from_digit(x: Digit) -> Self {
        let mut tmp = ExtAwi::zero(bw(BITS));
        tmp.digit_(x);
        tmp
    }
}

impl From<bool> for ExtAwi {
    fn from(x: bool) -> ExtAwi {
        let mut tmp = ExtAwi::zero(bw(1));
        tmp.bool_(x);
        tmp
    }
}

macro_rules! extawi_from {
    ($($ty:ident, $assign:ident);*;) => {
        $(
            impl From<$ty> for ExtAwi {
                fn from(x: $ty) -> Self {
                    let mut tmp = ExtAwi::zero(bw($ty::BITS as usize));
                    tmp.$assign(x);
                    tmp
                }
            }
        )*
    };
}

extawi_from!(
    u8, u8_;
    u16, u16_;
    u32, u32_;
    u64, u64_;
    u128, u128_;
    usize, usize_;
    i8, i8_;
    i16, i16_;
    i32, i32_;
    i64, i64_;
    i128, i128_;
    isize, isize_;
);
