use alloc::alloc::{alloc, alloc_zeroed, dealloc, Layout};
use core::{
    borrow::{Borrow, BorrowMut},
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem,
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Index, IndexMut, RangeFull},
    ptr,
    ptr::NonNull,
};

use awint_core::Bits;
use const_fn::const_fn;

use crate::awint_internals::*;

/// We use a `union` so that we can handle any difference in size and alignment
/// between a `Digit` and `*const Digit`. In the common case on most
/// architectures, this is simply `usize` sized and aligned which eliminates
/// overhead. We do not use a `NonNull` for `_ext` since it is in a union with
/// something that can be zero.
union InlOrExt {
    _inl: Digit,
    _ext: *const Digit,
}

/// An arbitrary width integer with manually controlled bitwidth. This is
/// different from [ExtAwi](crate::ExtAwi) and [InlAwi](awint_core::InlAwi) in
/// that it has a capacity, meaning that its bitwidth can be changed without
/// reallocation if the capacity is large enough.
///
/// Small bitwidths can be stored inline by this struct without any allocation,
/// which greatly helps cases where only a few of the `Awi`s are large.
///
/// This struct implements `Deref<Target = Bits>`, see the main documentation of
/// [Bits](awint_core::Bits) for more. There are also some functions that
/// `InlAwi` and `ExtAwi` do not implement, namely some bitwidth changing
/// functions.
///
/// See the crate level documentation of `awint_macros` for more macros and
/// information.
#[repr(C)]
pub struct Awi {
    /// # Design
    ///
    /// In any possible design we need `_cap` to keep information about the
    /// allocation layout. `_cap` is in units of bytes so that `Layout`s can use
    /// its value directly. We differentiate between inline and external
    /// allocation mode by setting `_cap` to zero when in inline mode. This
    /// is both semantically ideal and is the fastest kind of comparison to
    /// make on almost all architectures.
    ///
    /// `_inl_or_ext` stores either an inline `Digit` (and not `usize` for cases
    /// where `BITS > USIZE_BITS`) or a `*const Digit` pointing to an external
    /// allocation
    ///
    /// The `_nzbw` gives the actual bitwidth within the capacity and supplies
    /// the `NonZero` we want for nich optimizations.
    ///
    /// `_boo` is just insurance that we have the right covariance and stuff
    ///
    /// Invariants:
    ///
    /// - If `_cap == 0`, only `_inl_or_ext._inl` is used, and `_nzbw <= BITS`
    /// - If `_cap != 0`, only `_inl_or_ext._ext` is used, pointing to an
    ///   allocation with `_cap` bytes and an aligned array of `Digit`s. `_cap *
    ///   8` must not overflow. `_nzbw <= _cap * 8`. The allocation must always
    ///   be fully initialized
    _inl_or_ext: InlOrExt,
    _nzbw: NonZeroUsize,
    _cap: usize,
    _boo: PhantomData<NonNull<Digit>>,
}

/// `Awi` is safe to send between threads since it does not own aliasing memory
/// and has no reference counting mechanism like `Rc`.
unsafe impl Send for Awi {}

/// `Awi` is safe to share between threads since it does not own aliasing memory
/// and has no mutable internal state like `Cell` or `RefCell`.
unsafe impl Sync for Awi {}

impl<'a> Awi {
    /// This stores up to a `BITS` bitwidth integer as represented by
    /// `digit` inline. Unused bits clearing is _not_ performed.
    ///
    /// # Safety
    ///
    /// `nzbw.get() <= BITS` must hold.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const unsafe fn inl_from_raw_parts(digit: Digit, nzbw: NonZeroUsize) -> Awi {
        debug_assert!(nzbw.get() <= BITS);
        Awi {
            _inl_or_ext: InlOrExt { _inl: digit },
            _nzbw: nzbw,
            _cap: 0,
            _boo: PhantomData,
        }
    }

    /// This uses `digits` as externally allocated bits for the `Awi`. Unused
    /// bits clearing is _not_ performed.
    ///
    /// # Safety
    ///
    /// `digits` and `nzbw` together as a pointer to a `Digit` array and a
    /// bitwidth must satisfy the raw invariants of `Bits`, except that there
    /// can be more than the minimum number of `Digit`s needed to store all bits
    /// (see bits.rs). `cap_in_bytes * 8` must not overflow.
    /// `(cap_in_bytes * 8) >= nzbw.get()` must hold so that there are at least
    /// as many capacity bits as bitwidth.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const unsafe fn ext_from_raw_parts(
        digits: *const Digit,
        nzbw: NonZeroUsize,
        cap_in_bytes: usize,
    ) -> Awi {
        debug_assert!(cap_in_bytes.checked_mul(8).is_some());
        // this also implies that `cap_in_bytes != 0`
        debug_assert!((cap_in_bytes * 8) >= nzbw.get());
        Awi {
            _inl_or_ext: InlOrExt { _ext: digits },
            _nzbw: nzbw,
            _cap: cap_in_bytes,
            _boo: PhantomData,
        }
    }

    /// Returns a reference to `self` in the form of `&Bits`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    const fn internal_as_ref(&'a self) -> &'a Bits {
        if self._cap == 0 {
            // Safety: for inline storage we get a reference to the `_inl` field of the
            // union. Since it is exactly one `Digit` and `_nzbw <= BITS`, we have something
            // that satisfies the invariants for `RawBits` and `Bits`.
            unsafe {
                let tmp: &Digit = &self._inl_or_ext._inl;
                let tmp: *const Digit = tmp;
                let tmp = tmp as *mut Digit;
                Bits::from_raw_parts(RawBits::from_raw_parts(
                    NonNull::new_unchecked(tmp),
                    self._nzbw,
                ))
            }
        } else {
            // Safety: for external storage we get a reference to the `_ext` field of the
            // union. By the invariants, it satisfies the raw invariants of `Bits`.
            unsafe {
                let tmp = self._inl_or_ext._ext;
                let tmp = tmp as *mut Digit;
                Bits::from_raw_parts(RawBits::from_raw_parts(
                    NonNull::new_unchecked(tmp),
                    self._nzbw,
                ))
            }
        }
    }

    /// Returns a reference to `self` in the form of `&mut Bits`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    const fn internal_as_mut(&'a mut self) -> &'a mut Bits {
        if self._cap == 0 {
            // Safety: for inline storage we get a reference to the `_inl` field of the
            // union. Since it is exactly one `Digit` and `_nzbw <= BITS`, we have something
            // that satisfies the invariants for `RawBits` and `Bits`.
            unsafe {
                let tmp: &mut Digit = &mut self._inl_or_ext._inl;
                let tmp: *const Digit = tmp;
                let tmp = tmp as *mut Digit;
                Bits::from_raw_parts_mut(RawBits::from_raw_parts(
                    NonNull::new_unchecked(tmp),
                    self._nzbw,
                ))
            }
        } else {
            // Safety: for external storage we get a reference to the `_ext` field of the
            // union. By the invariants, it satisfies the raw invariants of `Bits`.
            unsafe {
                let tmp = self._inl_or_ext._ext;
                let tmp = tmp as *mut Digit;
                Bits::from_raw_parts_mut(RawBits::from_raw_parts(
                    NonNull::new_unchecked(tmp),
                    self._nzbw,
                ))
            }
        }
    }

    /// Returns the bitwidth of this `Awi` as a `NonZeroUsize`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn nzbw(&self) -> NonZeroUsize {
        self._nzbw
    }

    /// Returns the bitwidth of this `Awi` as a `usize`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn bw(&self) -> usize {
        self._nzbw.get()
    }

    /// Returns the capacity of this `Awi` in bits
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn capacity(&self) -> NonZeroUsize {
        if self._cap == 0 {
            // Safety: `BITS` is nonzero
            unsafe { NonZeroUsize::new_unchecked(BITS) }
        } else {
            // Safety: `self._cap * 8` is nonzero and cannot overflow to zero because of the
            // invariants
            unsafe { NonZeroUsize::new_unchecked(self._cap * 8) }
        }
    }

    /// Returns the `Layout` of the allocation if this `Awi` is externally
    /// allocated, otherwise returns `None` when inline.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn layout(&self) -> Option<Layout> {
        if self._cap == 0 {
            None
        } else {
            // Safety: `_cap` has the exact number of bytes of the allocation
            unsafe {
                Some(Layout::from_size_align_unchecked(
                    self._cap,
                    mem::align_of::<Digit>(),
                ))
            }
        }
    }

    //from_bits

    /// Zero-value construction with bitwidth `w`
    pub fn zero(w: NonZeroUsize) -> Self {
        if w.get() <= BITS {
            // Safety: the bitwidth is no larger than `BITS`
            unsafe { Awi::inl_from_raw_parts(0, w) }
        } else {
            // Safety: we allocate for a capacity that can store `w`. We use `size_in_bytes`
            // for `cap_in_bytes`.
            unsafe {
                let size_in_digits = total_digits(w).get();
                let size_in_bytes = size_in_digits * mem::size_of::<Digit>();
                let layout =
                    Layout::from_size_align_unchecked(size_in_bytes, mem::align_of::<Digit>());
                let ptr: *mut Digit = alloc_zeroed(layout).cast();
                Awi::ext_from_raw_parts(ptr, w, size_in_bytes)
            }
        }
    }

    //from_bits_with_capacity
    //zero_with_capacity

    // Increases capacity if necessary, otherwise just changes the bitwidth and
    // overwrites nothing, even previously set bits in the capacity
    // internal_resize()

    // Reserves capacity for at least `additional` more bits. More bits than
    // requested may be allocated.
    //
    // # Panics
    //
    // Panics if the new capacity exceeds `usize::MAX` bits
    //reserve(&mut self, additional: usize)

    //shrink_to(&mut self, min_capacity: NonZeroUsize)
    //shrink_to_fit(&mut self)

    // Not the same as `Bits::resize_`.
    //resize(&mut self, new_bitwidth: NonZeroUsize)
    //resize_unsigned()

    /// Used by `awint_macros` in avoiding a `NonZeroUsize` dependency
    #[doc(hidden)]
    pub fn panicking_zero(w: usize) -> Self {
        Self::zero(NonZeroUsize::new(w).unwrap())
    }
}

impl Drop for Awi {
    fn drop(&mut self) {
        if let Some(layout) = self.layout() {
            // Safety: deallocates our layout at the right pointer
            unsafe {
                dealloc(self._inl_or_ext._ext as *mut u8, layout);
            }
        }
    }
}

impl Clone for Awi {
    /// The capacity of the cloned `Awi` can be reduced to the minimum required
    /// for `self.nzbw()`
    fn clone(&self) -> Awi {
        if self._cap == 0 {
            // Safety: we copy the inline digit
            unsafe {
                Awi::inl_from_raw_parts(self._inl_or_ext._inl, self._nzbw)
                // we do not have to clear unused bits since `_inl` is a single
                // `Digit` and should already be cleared.
            }
        } else if self._nzbw.get() <= BITS {
            // we already checked for `self._cap == 0`, so we must read from the allocation
            // and switch to being inline

            // Safety: we use a digit and a bitwidth no more than `BITS` in size
            unsafe {
                let digit = self.internal_as_ref().to_digit();
                Awi::inl_from_raw_parts(digit, self._nzbw)
            }
        } else {
            // Safety: We create enough capacity, use the right alignment, initialize the
            // whole allocation, and use `size_in_bytes`.
            unsafe {
                let size_in_digits = total_digits(self._nzbw).get();
                let size_in_bytes = size_in_digits * mem::size_of::<Digit>();
                let layout =
                    Layout::from_size_align_unchecked(size_in_bytes, mem::align_of::<Digit>());
                let dst: *mut Digit = alloc(layout).cast();
                ptr::copy_nonoverlapping(self._inl_or_ext._ext, dst, size_in_digits);
                Awi::ext_from_raw_parts(dst, self._nzbw, size_in_bytes)
            }
        }
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl PartialEq for Awi {
    fn eq(&self, rhs: &Self) -> bool {
        self.as_ref() == rhs.as_ref()
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl Eq for Awi {}

#[cfg(feature = "zeroize_support")]
impl zeroize::Zeroize for Awi {
    fn zeroize(&mut self) {
        self.as_mut().zeroize()
    }
}

macro_rules! impl_fmt {
    ($($ty:ident)*) => {
        $(
            /// Forwards to the corresponding impl for `Bits`
            impl fmt::$ty for Awi {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::$ty::fmt(self.as_ref(), f)
                }
            }
        )*
    };
}

impl_fmt!(Debug Display LowerHex UpperHex Octal Binary);

impl Hash for Awi {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl Deref for Awi {
    type Target = Bits;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.internal_as_ref()
    }
}

impl DerefMut for Awi {
    #[inline]
    fn deref_mut(&mut self) -> &mut Bits {
        self.internal_as_mut()
    }
}

impl Index<RangeFull> for Awi {
    type Output = Bits;

    #[inline]
    fn index(&self, _i: RangeFull) -> &Bits {
        self
    }
}

impl Borrow<Bits> for Awi {
    #[inline]
    fn borrow(&self) -> &Bits {
        self
    }
}

impl AsRef<Bits> for Awi {
    #[inline]
    fn as_ref(&self) -> &Bits {
        self
    }
}

impl IndexMut<RangeFull> for Awi {
    #[inline]
    fn index_mut(&mut self, _i: RangeFull) -> &mut Bits {
        self
    }
}

impl BorrowMut<Bits> for Awi {
    #[inline]
    fn borrow_mut(&mut self) -> &mut Bits {
        self
    }
}

impl AsMut<Bits> for Awi {
    #[inline]
    fn as_mut(&mut self) -> &mut Bits {
        self
    }
}

macro_rules! awi_from_ty {
    ($($ty:ident $from:ident $assign:ident);*;) => {
        $(
            /// Creates an `Awi` with the same bitwidth and bits as the integer
            pub fn $from(x: $ty) -> Self {
                let mut tmp = Awi::zero(bw($ty::BITS as usize));
                tmp.$assign(x);
                tmp
            }
        )*
    };
}

impl Awi {
    awi_from_ty!(
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

    /// Creates an `Awi` with one bit set to this `bool`
    pub fn from_bool(x: bool) -> Self {
        let mut tmp = Awi::zero(bw(1));
        tmp.bool_(x);
        tmp
    }

    /// Creates an `Awi` with the same bitwidth and bits as the integer
    pub fn from_digit(x: Digit) -> Self {
        let mut tmp = Awi::zero(bw(BITS));
        tmp.digit_(x);
        tmp
    }
}

impl From<bool> for Awi {
    fn from(x: bool) -> Awi {
        let mut tmp = Awi::zero(bw(1));
        tmp.bool_(x);
        tmp
    }
}

macro_rules! awi_from {
    ($($ty:ident, $assign:ident);*;) => {
        $(
            impl From<$ty> for Awi {
                fn from(x: $ty) -> Self {
                    let mut tmp = Awi::zero(bw($ty::BITS as usize));
                    tmp.$assign(x);
                    tmp
                }
            }
        )*
    };
}

awi_from!(
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
