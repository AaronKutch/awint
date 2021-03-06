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
use awint_internals::*;
use const_fn::const_fn;

#[inline]
pub(crate) const fn layout(bw: NonZeroUsize) -> Layout {
    unsafe {
        // Safety: this produces the exact number of bytes needed to satisfy the raw
        // invariants of `Bits`.
        Layout::from_size_align_unchecked(
            (regular_digits(bw) + 1) * mem::size_of::<usize>(),
            mem::align_of::<usize>(),
        )
    }
}

/// An arbitrary width integer with manually controlled bitwidth. Most
/// arithmetic is wrapping like Rust's integers. All reallocations are
/// explicit.
///
/// ```
/// #![feature(const_mut_refs)]
/// use awint::{bw, inlawi, Bits, ExtAwi, InlAwi};
///
/// const fn example(x0: &mut Bits, x1: &Bits) {
///     // when dealing with `Bits` with different bitwidths, use the
///     // `_resize_assign` functions or the concatenations of components
///     // macros with unbounded fillers from `awint_macros`
///     x0.sign_resize_assign(x1);
///     // multiply in place by 2 for an example
///     x0.short_cin_mul(0, 2);
/// }
///
/// // using `bw` function for quick `NonZeroUsize` construction from a literal
/// let mut awi0 = ExtAwi::zero(bw(100));
/// // constructing an `ExtAwi` from an `InlAwi`
/// let awi1 = ExtAwi::from(inlawi!(-123i16));
/// // `ExtAwi` implements `DerefMut`, but a mutable `Bits` reference can also
/// // be acquired like this.
/// let x0: &mut Bits = awi0.const_as_mut();
/// assert!(x0.is_zero());
/// example(x0, awi1.const_as_ref());
/// assert_eq!(x0, inlawi!(-246i100).const_as_ref());
/// // you can freely mix references originating from both `ExtAwi` and `InlAwi`
/// example(x0, inlawi!(0x10u16).const_as_ref());
/// assert_eq!(awi0, ExtAwi::from(inlawi!(0x20u100)));
/// ```
#[repr(transparent)]
pub struct ExtAwi {
    raw: NonNull<Bits>,
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
    /// all digits initialized. The metadata digit should be set to the
    /// bitwidth. `raw_len` should correspond to the raw length (including
    /// metadata) of the corresponding `Bits`.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const unsafe fn from_raw_parts(ptr: *mut usize, raw_len: usize) -> ExtAwi {
        // Safety: The requirements on this function satisfies
        // `Bits::from_raw_parts_mut`.
        unsafe {
            ExtAwi {
                raw: NonNull::new_unchecked(Bits::from_raw_parts_mut(ptr, raw_len)),
            }
        }
    }

    /// Returns a reference to `self` in the form of `&Bits`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn const_as_ref(&'a self) -> &'a Bits {
        // `as_ref` on NonNull is not const yet, so we have to use transmute.
        // Safety: The explicit lifetimes make sure they do not become unbounded.
        unsafe { mem::transmute::<NonNull<Bits>, &Bits>(self.raw) }
    }

    /// Returns a reference to `self` in the form of `&mut Bits`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn const_as_mut(&'a mut self) -> &'a mut Bits {
        // Safety: The explicit lifetimes make sure they do not become unbounded.
        unsafe { mem::transmute::<NonNull<Bits>, &mut Bits>(self.raw) }
    }

    /// Returns the bitwidth of this `ExtAwi` as a `NonZeroUsize`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn nzbw(&self) -> NonZeroUsize {
        self.const_as_ref().nzbw()
    }

    /// Returns the bitwidth of this `ExtAwi` as a `usize`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn bw(&self) -> usize {
        self.const_as_ref().bw()
    }

    /// Returns the exact number of `usize` digits needed to store all bits.
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn len(&self) -> usize {
        self.const_as_ref().len()
    }

    /// Returns the total length of the underlying raw array in `usize`s
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn raw_len(&self) -> usize {
        self.const_as_ref().raw_len()
    }

    /// Returns a raw pointer to the underlying bit storage. The caller must
    /// ensure that the `ExtAwi` outlives the pointer this function returns,
    /// and that the `ExtAwi` is not reallocated. The underlying memory should
    /// never be written to.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub fn as_ptr(&self) -> *const usize {
        self.const_as_ref().as_ptr()
    }

    /// Returns a raw pointer to the underlying bit storage. The caller must
    /// ensure that the `ExtAwi` outlives the pointer this function returns,
    /// and that the `ExtAwi` is not reallocated.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub fn as_mut_ptr(&mut self) -> *mut usize {
        self.const_as_mut().as_mut_ptr()
    }

    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn layout(&self) -> Layout {
        layout(self.nzbw())
    }

    /// Creates an `ExtAwi` from copying a `Bits` reference. The same
    /// functionality is provided by an `From<&Bits>` implementation for
    /// `ExtAwi`.
    pub fn from_bits(bits: &Bits) -> ExtAwi {
        let mut tmp = ExtAwi::zero(bits.nzbw());
        tmp.const_as_mut().copy_assign(bits).unwrap();
        tmp
    }

    /// Zero-value construction with bitwidth `bw`
    pub fn zero(bw: NonZeroUsize) -> Self {
        // Safety: This satisfies `ExtAwi::from_raw_parts`
        unsafe {
            let ptr: *mut usize = alloc_zeroed(layout(bw)).cast();
            // set bitwidth
            ptr.add(regular_digits(bw)).write(bw.get());
            ExtAwi::from_raw_parts(ptr, regular_digits(bw) + 1)
        }
    }

    /// Unsigned-maximum-value construction with bitwidth `bw`
    pub fn umax(bw: NonZeroUsize) -> Self {
        // Safety: This satisfies `ExtAwi::from_raw_parts`
        let mut x = unsafe {
            let ptr: *mut usize = alloc(layout(bw)).cast();
            // initialize everything except for the bitwidth
            ptr.write_bytes(u8::MAX, regular_digits(bw));
            // set bitwidth
            ptr.add(regular_digits(bw)).write(bw.get());
            ExtAwi::from_raw_parts(ptr, regular_digits(bw) + 1)
        };
        x.const_as_mut().clear_unused_bits();
        x
    }

    /// Signed-maximum-value construction with bitwidth `bw`
    pub fn imax(bw: NonZeroUsize) -> Self {
        let mut awi = Self::umax(bw);
        *awi.const_as_mut().last_mut() = (isize::MAX as usize) >> awi.const_as_ref().unused();
        awi
    }

    /// Signed-minimum-value construction with bitwidth `bw`
    pub fn imin(bw: NonZeroUsize) -> Self {
        let mut awi = Self::zero(bw);
        *awi.const_as_mut().last_mut() = (isize::MIN as usize) >> awi.const_as_ref().unused();
        awi
    }

    /// Unsigned-one-value construction with bitwidth `bw`
    pub fn uone(bw: NonZeroUsize) -> Self {
        let mut awi = Self::zero(bw);
        *awi.const_as_mut().first_mut() = 1;
        awi
    }

    /// Used by `awint_macros` in avoiding a `NonZeroUsize` dependency
    #[doc(hidden)]
    pub fn panicking_zero(bw: usize) -> Self {
        Self::zero(NonZeroUsize::new(bw).unwrap())
    }

    /// Used by `awint_macros` in avoiding a `NonZeroUsize` dependency
    #[doc(hidden)]
    pub fn panicking_umax(bw: usize) -> Self {
        Self::umax(NonZeroUsize::new(bw).unwrap())
    }

    /// Used by `awint_macros` in avoiding a `NonZeroUsize` dependency
    #[doc(hidden)]
    pub fn panicking_imax(bw: usize) -> Self {
        Self::imax(NonZeroUsize::new(bw).unwrap())
    }

    /// Used by `awint_macros` in avoiding a `NonZeroUsize` dependency
    #[doc(hidden)]
    pub fn panicking_imin(bw: usize) -> Self {
        Self::imin(NonZeroUsize::new(bw).unwrap())
    }

    /// Used by `awint_macros` in avoiding a `NonZeroUsize` dependency
    #[doc(hidden)]
    pub fn panicking_uone(bw: usize) -> Self {
        Self::uone(NonZeroUsize::new(bw).unwrap())
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
        // Safety: The digits and metadata are copied wholesale to the correct layout
        unsafe {
            let dst = alloc(self.layout()).cast();
            ptr::copy_nonoverlapping(self.as_ptr(), dst, self.raw_len());
            ExtAwi::from_raw_parts(dst, self.raw_len())
        }
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl PartialEq for ExtAwi {
    fn eq(&self, rhs: &Self) -> bool {
        self.const_as_ref() == rhs.const_as_ref()
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl Eq for ExtAwi {}

macro_rules! impl_fmt {
    ($($ty:ident)*) => {
        $(
            /// Forwards to the corresponding impl for `Bits`
            impl fmt::$ty for ExtAwi {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::$ty::fmt(self.const_as_ref(), f)
                }
            }
        )*
    };
}

impl_fmt!(Debug Display LowerHex UpperHex Octal Binary);

impl Hash for ExtAwi {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.const_as_ref().hash(state);
    }
}

impl Deref for ExtAwi {
    type Target = Bits;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.const_as_ref()
    }
}

impl DerefMut for ExtAwi {
    #[inline]
    fn deref_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

/// A quick way of getting an `&Bits` reference
impl Index<RangeFull> for ExtAwi {
    type Output = Bits;

    #[inline]
    fn index(&self, _i: RangeFull) -> &Bits {
        self.const_as_ref()
    }
}

impl Borrow<Bits> for ExtAwi {
    #[inline]
    fn borrow(&self) -> &Bits {
        self.const_as_ref()
    }
}

impl AsRef<Bits> for ExtAwi {
    #[inline]
    fn as_ref(&self) -> &Bits {
        self.const_as_ref()
    }
}

/// A quick way of getting an `&mut Bits` reference
impl IndexMut<RangeFull> for ExtAwi {
    #[inline]
    fn index_mut(&mut self, _i: RangeFull) -> &mut Bits {
        self.const_as_mut()
    }
}

impl BorrowMut<Bits> for ExtAwi {
    #[inline]
    fn borrow_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

impl AsMut<Bits> for ExtAwi {
    #[inline]
    fn as_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

/// Creates an `ExtAwi` from copying a `Bits` reference
impl From<&Bits> for ExtAwi {
    fn from(bits: &Bits) -> ExtAwi {
        let mut tmp = ExtAwi::zero(bits.nzbw());
        tmp.const_as_mut().copy_assign(bits).unwrap();
        tmp
    }
}

/// Creates an `ExtAwi` from copying an `InlAwi`
impl<const BW: usize, const LEN: usize> From<InlAwi<BW, LEN>> for ExtAwi {
    fn from(awi: InlAwi<BW, LEN>) -> ExtAwi {
        let mut tmp = ExtAwi::zero(awi.nzbw());
        tmp.const_as_mut().copy_assign(awi.const_as_ref()).unwrap();
        tmp
    }
}

macro_rules! extawi_from_ty {
    ($($ty:ident $from:ident $assign:ident);*;) => {
        $(
            /// Creates an `ExtAwi` with the same bitwidth and bits as the integer
            pub fn $from(x: $ty) -> Self {
                let mut tmp = ExtAwi::zero(bw($ty::BITS as usize));
                tmp.const_as_mut().$assign(x);
                tmp
            }
        )*
    };
}

impl ExtAwi {
    extawi_from_ty!(
        u8 from_u8 u8_assign;
        u16 from_u16 u16_assign;
        u32 from_u32 u32_assign;
        u64 from_u64 u64_assign;
        u128 from_u128 u128_assign;
        usize from_usize usize_assign;
        i8 from_i8 i8_assign;
        i16 from_i16 i16_assign;
        i32 from_i32 i32_assign;
        i64 from_i64 i64_assign;
        i128 from_i128 i128_assign;
        isize from_isize isize_assign;
    );

    /// Creates an `ExtAwi` with one bit set to this `bool`
    pub fn from_bool(x: bool) -> Self {
        let mut tmp = ExtAwi::zero(bw(1));
        tmp.const_as_mut().bool_assign(x);
        tmp
    }
}

impl From<bool> for ExtAwi {
    fn from(x: bool) -> ExtAwi {
        let mut tmp = ExtAwi::zero(bw(1));
        tmp.const_as_mut().bool_assign(x);
        tmp
    }
}

macro_rules! extawi_from {
    ($($ty:ident, $assign:ident);*;) => {
        $(
            impl From<$ty> for ExtAwi {
                fn from(x: $ty) -> Self {
                    let mut tmp = ExtAwi::zero(bw($ty::BITS as usize));
                    tmp.const_as_mut().$assign(x);
                    tmp
                }
            }
        )*
    };
}

extawi_from!(
    u8, u8_assign;
    u16, u16_assign;
    u32, u32_assign;
    u64, u64_assign;
    u128, u128_assign;
    usize, usize_assign;
    i8, i8_assign;
    i16, i16_assign;
    i32, i32_assign;
    i64, i64_assign;
    i128, i128_assign;
    isize, isize_assign;
);
