use core::{num::NonZeroUsize, ptr::NonNull};

use const_fn::const_fn;

use crate::{Digit, BITS};

// Note: this crate cannot depend on `alloc`,
// so the `layout` function is contained in `awint_extawi`

/// A hack that is very close to the ideal of having a `(NonNull<T>,
/// NonZeroUsize)` custom DST for `Bits` to use.
///
/// # Note
///
/// This currently requires `-Zmiri-tree-borrows` in order for MIRI to accept
/// it.
// TODO remove this note when tree borrows is stable or replaced
#[repr(C)]
pub struct CustomDst<T> {
    _dst: [[T; 0]],
}

impl<T> CustomDst<T> {
    /// In order to use this correctly, create a struct that just has
    /// `CustomDst<SpecifiedSizedType>` (plus potentially a `PhantomData` but no
    /// non-ZST fields) and is `#[repr(C)]`. Use this function to store a
    /// `T`-aligned and nonnull pointer, and an arbitrary `usize`. Transform
    /// into the struct with `unsafe { &*(custom_dst as *mut Bits) }` or `unsafe
    /// { &mut *(custom_dst as *mut Bits) }` and make sure to use that fat
    /// pointer `as` cast instead of a transmute. Make sure to follow lifetime
    /// and aliasing rules.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn from_raw_parts(
        t_aligned_and_nonnull_ptr: NonNull<T>,
        arbitrary_usize: usize,
    ) -> *mut CustomDst<T> {
        // Safety: `[T; 0]` has the same alignment as `T`, and `T` is covariant with
        // `[T; 0]` (https://doc.rust-lang.org/reference/subtyping.html)
        let tmp0: NonNull<[T; 0]> = t_aligned_and_nonnull_ptr.cast::<[T; 0]>();
        // Safety: There is no allocated data from the slice's perspective since `[T;
        // 0]` is a ZST, the only requirement was that `tmp0` was nonnull and
        // aligned properly. `len * mem::size_of::<[T; 0]>() == len * 0 == 0`, so the
        // expression can never exceed `isize::MAX` and `arbitrary_usize` can be
        // anything. This is an entirely valid slice with no insta UB or UB even
        // if we try to use it as a normal slice.
        let tmp1: NonNull<[[T; 0]]> = NonNull::slice_from_raw_parts(tmp0, arbitrary_usize);
        // Safety: `CustomDst<T>` is `#[repr(C)]` over `[[T; 0]]` and we use a fat
        // pointer cast.
        let tmp2: *mut CustomDst<T> = tmp1.as_ptr() as *mut CustomDst<T>;
        tmp2
    }

    /// Retrieves the `t_aligned_and_nonnull_ptr` this was constructed from
    pub const fn as_ptr(&self) -> *const T {
        // Safety: The pointer has the same provenance as the
        // `t_aligned_and_nonnull_ptr` it was originally created from
        let tmp: *const [T; 0] = self._dst.as_ptr();
        tmp as *const T
    }

    #[const_fn(cfg(feature = "const_support"))]
    pub const fn as_mut_ptr(&mut self) -> *mut T {
        // Safety: The pointer has the same provenance as the
        // `t_aligned_and_nonnull_ptr` it was originally created from
        let tmp: *mut [T; 0] = self._dst.as_mut_ptr();
        tmp as *mut T
    }

    /// Retrieves the `arbitrary_usize` this was constructed from
    pub const fn get_usize(&self) -> usize {
        self._dst.len()
    }
}

#[repr(C)]
pub struct RawBits {
    _digits: NonNull<Digit>,
    _bw: NonZeroUsize,
}

impl RawBits {
    pub const fn from_raw_parts(digits: NonNull<Digit>, bw: NonZeroUsize) -> Self {
        Self {
            _digits: digits,
            _bw: bw,
        }
    }

    pub const fn as_non_null_ptr(&self) -> NonNull<Digit> {
        self._digits
    }

    #[inline]
    pub const fn nzbw(&self) -> NonZeroUsize {
        self._bw
    }

    #[inline]
    pub const fn bw(&self) -> usize {
        self.nzbw().get()
    }

    /// Returns the number of _whole_ digits in `self` (not including the last
    /// digit if it has unused bits)
    #[inline]
    pub const fn digits(&self) -> usize {
        self.bw().wrapping_shr(BITS.trailing_zeros())
    }

    /// Returns the number of extra bits of `self`. When `self.bw()` is an exact
    /// multiple of `BITS`, there are no extra bits and `self.extra() == 0`. If
    /// `self.bw()` is not an exact multiple, there are some unused bits in the
    /// last digit, and the extra bits are the number of _used_ bits in the last
    /// digit.
    #[inline]
    pub const fn extra(&self) -> usize {
        self.bw() & (BITS - 1)
    }

    /// Returns the total number of digits in the raw representation of `self`.
    #[inline]
    pub const fn total_digits(&self) -> NonZeroUsize {
        // Safety: if `self.digits()` is zero, `self.extra()` must be nonzero
        unsafe {
            NonZeroUsize::new_unchecked(self.digits().wrapping_add((self.extra() != 0) as usize))
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawStackBits<const BW: usize, const LEN: usize> {
    pub _digits: [Digit; LEN],
}

impl<const BW: usize, const LEN: usize> RawStackBits<BW, LEN> {
    /// Checks that the `BW` and `LEN` values are valid for `RawStackBits`.
    ///
    /// # Panics
    ///
    /// If `BW == 0`, `LEN == 0`, or the bitwidth is outside the
    /// range `(((LEN - 1)*BITS) + 1)..=(LEN*BITS)`
    pub const fn _assert_invariants() {
        if BW == 0 {
            panic!("Tried to create `RawStackBits<BW, LEN>` with `BW == 0`")
        }
        if LEN == 0 {
            panic!("Tried to create `RawStackBits<BW, LEN>` with `LEN == 0`")
        }
        if BW <= ((LEN - 1) * BITS) {
            panic!("Tried to create `RawStackBits<BW, LEN>` with `BW <= Digit::BITS * (LEN - 1)`")
        }
        if BW > (LEN * BITS) {
            panic!("Tried to create `RawStackBits<BW, LEN>` with `BW > Digit::BITS * LEN`")
        }
    }

    pub const fn nzbw() -> NonZeroUsize {
        Self::_assert_invariants();
        // Safety: `_assert_invariants` above panics if `BW == 0`
        unsafe { NonZeroUsize::new_unchecked(BW) }
    }

    pub const fn bw() -> usize {
        Self::nzbw().get()
    }

    /// Returns the number of _whole_ digits in `self` (not including the last
    /// digit if it has unused bits)
    pub const fn digits() -> usize {
        Self::bw().wrapping_shr(BITS.trailing_zeros())
    }

    /// Returns the number of extra bits of `self`. When `self.bw()` is an exact
    /// multiple of `BITS`, there are no extra bits and `self.extra() == 0`. If
    /// `self.bw()` is not an exact multiple, there are some unused bits in the
    /// last digit, and the extra bits are the number of _used_ bits in the last
    /// digit.
    pub const fn extra() -> usize {
        Self::bw() & (BITS - 1)
    }

    /// Returns the total number of digits in the raw representation of `self`.
    pub const fn total_digits() -> NonZeroUsize {
        // Safety: if `Self::digits()` is zero, `Self::extra()` must be nonzero
        unsafe {
            NonZeroUsize::new_unchecked(Self::digits().wrapping_add((Self::extra() != 0) as usize))
        }
    }

    pub const fn zero() -> Self {
        Self::_assert_invariants();
        Self { _digits: [0; LEN] }
    }

    /// Note: this does not unset unused bits
    pub const fn all_set() -> Self {
        Self::_assert_invariants();
        Self { _digits: [!0; LEN] }
    }

    #[const_fn(cfg(feature = "const_support"))]
    pub const fn to_raw_bits(&self) -> RawBits {
        // Safety: we can get a `NonNull` from an array address
        unsafe {
            RawBits::from_raw_parts(
                NonNull::new_unchecked(self._digits.as_ptr() as *mut Digit),
                Self::nzbw(),
            )
        }
    }
}
