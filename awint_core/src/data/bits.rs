//! ## Basic Invariants
//!
//! - `Bits` have a nonzero bit-width specified in a `NonZeroUsize`. Being
//!   nonzero, it eliminates several edge cases and ambiguities this crate would
//!   have to handle.
//! - `Bits` are stored in little endian order with the same requirements as
//!   `[Digit]`. The number of `Digit`s is the minimum needed to store all bits.
//!   If bitwidth is not a multiple of `Digit::BITS`, then there will be some
//!   unused bits in the last `Digit`. For example, a bitwidth of of 100 bits
//!   takes up 2 digits (if `Digit::BITS == 64`): 64 bits in the first digit, 36
//!   bits in the least significant bits of the second, and 28 unused bits in
//!   the remaining bits of the second.
//! - Unused bits are zeroed. Note that this is not a safety critical invariant.
//!   Setting unused bits via `Bits::as_mut_slice` or `Bits::last_mut` will not
//!   cause Rust U.B., but it may result in arithmetically incorrect results or
//!   panics from the functions on `Bits`. Arbitrary bits can in the last digit
//!   can be set temporarily, but [Bits::clear_unused_bits] should be run before
//!   reaching a function that expects all these invariants to hold.

use core::{
    fmt,
    hash::{Hash, Hasher},
    num::NonZeroUsize,
    ops::Range,
    ptr::{self, NonNull},
};

use awint_internals::*;
use const_fn::const_fn;

/// A reference to the bits in an `InlAwi`, `ExtAwi`, `Awi`, or other backing
/// construct. If a function is written just in terms of `Bits`, it can work on
/// mixed references to any of the storage structs and wrappers like `FP<B>`.
/// `const` big integer arithmetic is possible if the backing type is `InlAwi`
/// and the "const_support" flag is enabled.
///
/// `Bits` do **not** know signedness. Instead, the methods on `Bits` are
/// specified to interpret the bits as unsigned or signed two's complement
/// integers. If a method's documentation does not mention signedness, it either
/// works for both kinds or views the bits as a plain bit string with no
/// integral properties.
///
/// See the [`awint_core` crate level documentation](crate) for understanding
/// two's complement and numerical limits.
///
/// # Note
///
/// Function names of the form `*_` with a trailing underscore are shorthand for
/// saying `*_assign`, which denotes an inplace assignment operation where the
/// left hand side is used as an input before being reassigned the value of the
/// output inplace. This is used instead of the typical 2-input 1-new-output
/// kind of function, because:
/// - `Bits` cannot allocate without choosing a storage type
/// - In most cases during the course of computation, one value will not be
///   needed after being used once as an input. It can take the left hand side
///   `self` value of these inplace assignment operations.
/// - For large bitwidth `Bits`, only two streams of addresses have to be
///   considered by the CPU
/// - In cases where we do need buffering, copying to some temporary is the
///   fastest kind of operation (and in the future an optimizing macro for this
///   is planned)
///
/// Unless otherwise specified, functions on `Bits` that return an `Option<()>`
/// return `None` if the input bitwidths are not equal to each other. The `Bits`
/// have been left unchanged if `None` is returned.
///
/// # Portability
///
/// This crate strives to maintain deterministic outputs across architectures
/// with different `usize::BITS`, `Digit::BITS`, and different endiannesses. The
/// [Bits::u8_slice_] function, the [Bits::to_u8_slice] functions, the
/// serialization impls enabled by `serde_support`, the strings produced by the
/// `const` serialization functions, and functions like `bits_to_string_radix`
/// in the `awint_ext` crate are all portable and should be used when sending
/// representations of `Bits` between architectures.
///
/// The `rand_` function enabled by `rand_support` uses a
/// deterministic byte oriented implementation to avoid portability issues as
/// long as the rng itself is portable.
///
/// The [core::hash::Hash] implementation is _not_ deterministic across
/// platforms and may not even be deterministic across compiler versions. This
/// is because of technical problems, and the standard library docs say it is
/// not intended to be portable anyway.
///
/// There are many functions that depend on `Digit`, `usize`, and
/// `NonZeroUsize`. In cases where the `usize` describes the bitwidth, a bit
/// shift, or a bit position, the user should not need to worry about
/// portability, since if the values are close to `usize::MAX`, the user is
/// already close to running out of possible memory any way.
///
/// There are a few usages of `Digit` that are actual
/// views into a contiguous range of bits inside `Bits`, such as
/// `Bits::as_slice`, `Bits::first`, and `Bits::get_digit` (which are all hidden
/// from the documentation, please refer to the source code of `bits.rs` if
/// needed). Most end users should not use these, since they have a strong
/// dependence on the size of `Digit`. These functions are actual views into the
/// inner building blocks of this crate that other functions are built around in
/// such a way that they are portable (e.g. the addition functions may
/// internally operate on differing numbers of `Digit`s depending on the
/// size of `Digit`, but the end result looks the same to users on different
/// architectures). The only reason these functions are exposed, is that someone
/// may want to write their own custom performant algorithms, and they want as
/// few abstractions as possible in the way.
///
/// Visible functions that are not portable in general, but always start from
/// the zeroeth bit or a given bit position like [Bits::digit_cin_mul_],
/// [Bits::digit_udivide_], or [Bits::digit_or_], are always
/// portable as long as the digit inputs and/or outputs are restricted to
/// `0..=u8::MAX`, or special care is taken.
// We don't need the `transparent` anymore, but we will keep it anyway in case of external users who
// may want to do something unsafe with `Bits`
#[repr(C)]
pub struct Bits {
    /// # Raw Invariants
    ///
    /// We have chosen `Bits` to be a DST in order to avoid double indirection
    /// (`&mut Bits` would be a pointer to a `Bits` struct which in turn had a
    /// pointer inside itself to the actual digits). A DST also lets us get
    /// around <https://github.com/rust-lang/rust/issues/57749> ,
    /// which is absolutely required for the macros.
    ///
    /// Until true custom DSTs are supported in Rust, I have found a workaround
    /// that avoids expensive metadata tricks that earlier `awint` versions
    /// had to do. The `CustomDst` stores a pointer to the digits, and stores
    /// the bitwidth instead of the digit length. The pointer must be nonnull
    /// and to an allocation array of `Digits`, with the length being equal to
    /// `awint_internals::total_digits(bitwidth)`. In other words, the
    /// invariants are the same as `std::slice::from_raw_parts_mut` except we
    /// store only the bitwidth and calculate the slice length through
    /// `total_digits`.
    _custom_dst: CustomDst<Digit>,
}

/// `Bits` is safe to send between threads since it does not own
/// aliasing memory and has no reference counting mechanism like `Rc`.
unsafe impl Send for Bits {}

/// `Bits` is safe to share between threads since it does not own
/// aliasing memory and has no mutable internal state like `Cell` or `RefCell`.
unsafe impl Sync for Bits {}

/// # Basic functions
impl<'a> Bits {
    /// # Safety
    ///
    /// `raw_bits` should satisfy the raw invariants of `Bits` (see bits.rs).
    /// `Bits` itself does not allocate or deallocate memory. It is expected
    /// that the caller had a struct with proper `Drop` implementation,
    /// created `Bits` from that struct, and insured that the struct is
    /// borrowed for the duration of the `Bits` lifetime. The memory
    /// referenced by `bits` must not be accessed through any other
    /// reference for the duration of lifetime `'a`.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const unsafe fn from_raw_parts(raw_bits: RawBits) -> &'a Self {
        // Safety: `Bits` follows the assumptions of `CustomDst`. The explicit
        // lifetimes make sure they do not become unbounded.
        let custom_dst =
            CustomDst::<Digit>::from_raw_parts(raw_bits.as_non_null_ptr(), raw_bits.bw());
        unsafe { &*(custom_dst as *const Bits) }
    }

    /// # Safety
    ///
    /// see [Bits::from_raw_parts]
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const unsafe fn from_raw_parts_mut(raw_bits: RawBits) -> &'a mut Self {
        // Safety: `Bits` follows the assumptions of `CustomDst`. The explicit
        // lifetimes make sure they do not become unbounded.
        let custom_dst =
            CustomDst::<Digit>::from_raw_parts(raw_bits.as_non_null_ptr(), raw_bits.bw());
        unsafe { &mut *(custom_dst as *mut Bits) }
    }

    /// Returns the argument of this function. This exists so that the macros in
    /// `awint_macros` work on all storage types and `Bits` without needing to
    /// determine the type.
    #[doc(hidden)]
    #[inline]
    #[must_use]
    pub const fn const_as_ref(&'a self) -> &'a Bits {
        self
    }

    /// Returns the argument of this function. This exists so that the macros in
    /// `awint_macros` work on all storage types and `Bits` without needing to
    /// determine the type.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn const_as_mut(&'a mut self) -> &'a mut Bits {
        self
    }

    /// Returns a raw pointer to the underlying bit storage. The caller must
    /// ensure that the `Bits` outlives the pointer this function returns.
    /// The underlying memory should never be written to.
    #[doc(hidden)]
    #[inline]
    #[must_use]
    pub const fn as_ptr(&self) -> *const Digit {
        self._custom_dst.as_ptr()
    }

    /// Returns a raw pointer to the underlying bit storage. The caller must
    /// ensure that the `Bits` outlives the pointer this function returns.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn as_mut_ptr(&mut self) -> *mut Digit {
        self._custom_dst.as_mut_ptr()
    }

    /// Returns the bitwidth as a `NonZeroUsize`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn nzbw(&self) -> NonZeroUsize {
        // Safety: `Bits::from_raw...` uses the `NonZeroUsize` of `RawBits` for setting
        // the `usize`
        unsafe { NonZeroUsize::new_unchecked(self._custom_dst.get_usize()) }
    }

    /// Returns the bitwidth as a `usize`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn bw(&self) -> usize {
        self.nzbw().get()
    }

    /// # Safety
    ///
    /// `i < self.total_digits()` should hold true
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const unsafe fn get_unchecked(&self, i: usize) -> Digit {
        debug_assert!(i < self.total_digits());
        // Safety: `i < self.total_digits()` means the access is within the slice
        unsafe { *self.as_ptr().add(i) }
    }

    /// # Safety
    ///
    /// `i < self.total_digits()` should hold true
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const unsafe fn get_unchecked_mut(&'a mut self, i: usize) -> &'a mut Digit {
        debug_assert!(i < self.total_digits());
        // Safety: The bounds of this are a subset of `raw_get_unchecked_mut`
        unsafe { &mut *self.as_mut_ptr().add(i) }
    }

    /// Returns the exact number of `Digit`s needed to store all bits.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn total_digits(&self) -> usize {
        total_digits(self.nzbw()).get()
    }

    /// Returns the number of unused bits.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn unused(&self) -> usize {
        if self.extra() == 0 {
            0
        } else {
            BITS - self.extra()
        }
    }

    /// Returns the number of extra bits, or `BITS - self.unused()`. If
    /// there are no unused bits, this is zero.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn extra(&self) -> usize {
        extra(self.nzbw())
    }

    /// Returns the first `Digit`
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn first(&self) -> Digit {
        // Safety: There is at least one digit since bitwidth has a nonzero invariant
        unsafe { self.get_unchecked(0) }
    }

    /// Returns a mutable reference to the first `Digit`
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn first_mut(&'a mut self) -> &'a mut Digit {
        // Safety: There is at least one digit since bitwidth has a nonzero invariant
        unsafe { self.get_unchecked_mut(0) }
    }

    /// Returns the last `Digit`
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn last(&self) -> Digit {
        // Safety: There is at least one digit since bitwidth has a nonzero invariant
        unsafe { self.get_unchecked(self.total_digits() - 1) }
    }

    /// Returns a mutable reference to the last `Digit`
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn last_mut(&'a mut self) -> &'a mut Digit {
        // Safety: There is at least one digit since bitwidth has a nonzero invariant
        unsafe { self.get_unchecked_mut(self.total_digits() - 1) }
    }

    /// Clears the unused bits. This is only needed if you are using certain
    /// hidden functions to write to the digits directly.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn clear_unused_bits(&mut self) {
        if self.extra() == 0 {
            return // There are no unused bits
        }
        *self.last_mut() &= MAX >> (BITS - self.extra());
    }

    /// Some functions cannot handle set unused bits, so this acts as a quick
    /// way to check if unused bits are indeed clear.
    ///
    /// # Panics
    ///
    /// Panics if unused bits are set.
    #[doc(hidden)]
    #[track_caller]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn assert_cleared_unused_bits(&self) {
        let one: Digit = 1;
        if (self.extra() != 0) && (self.last() >= one.wrapping_shl(self.extra() as u32)) {
            panic!(
                "unused bits are set in a `Bits` struct, they have been set with one of the \
                 hidden functions and not properly unset with `Bits::clear_unused_bits`"
            );
        }
    }

    /// This is an extremely unsafe function that is intended only to be used
    /// through the `subdigits_mut` macro.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const unsafe fn internal_subdigits_mut(
        this: &'a mut Self,
        range_start: usize,
        range_end: usize,
    ) -> &'a mut Self {
        // Safety: `range_start < range_end` and is a nonzero digit range in another
        // `Bits`
        unsafe {
            let new_start: *mut Digit = this.as_mut_ptr().add(range_start);
            let new_nzbw =
                NonZeroUsize::new_unchecked(range_end.wrapping_sub(range_start).wrapping_mul(BITS));
            Bits::from_raw_parts_mut(RawBits::from_raw_parts(
                NonNull::new_unchecked(new_start),
                new_nzbw,
            ))
        }
    }

    /// Returns a reference to all of the underlying bits of `self`, including
    /// unused bits.
    ///
    /// # Note
    ///
    /// If the `Bits` has unused bits, those bits will always be set to zero,
    /// even if the `Bits` are intended to be a sign extended integer.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn as_slice(&'a self) -> &'a [Digit] {
        // Safety: `Bits` already follows standard slice initialization invariants. This
        // acquires a subslice that uses the total length instead of the bitwidth.
        unsafe { &*ptr::slice_from_raw_parts(self.as_ptr(), self.total_digits()) }
    }

    /// Returns a mutable reference to all of the underlying bits of `self`,
    /// including unused bits.
    ///
    /// # Note
    ///
    /// Unused bits can be temporarily set but should be cleared before they
    /// are used by another function that expects the standard `Bits` invariants
    /// to be upheld. Set unused bits will not cause Rust undefined behavior,
    /// but may cause incorrect arithmetical results or panics.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn as_mut_slice(&'a mut self) -> &'a mut [Digit] {
        // Safety: The same as `as_slice`
        unsafe { &mut *ptr::slice_from_raw_parts_mut(self.as_mut_ptr(), self.total_digits()) }
    }

    /// Returns a reference to the underlying bits of `self`, including unused
    /// bits (which occur if `self.bw()` is not a multiple of `Digit::BITS`).
    ///
    /// # Note
    ///
    /// If the `Bits` has unused bits, those bits will always be set to zero,
    /// even if the `Bits` are intended to be a sign extended integer.
    ///
    /// # Portability
    ///
    /// This function is highly non-portable across architectures, see the
    /// source code of [Bits::rand_] for how to handle this
    #[doc(hidden)]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn as_bytes_full_width_nonportable(&'a self) -> &'a [u8] {
        // Previously, this function used to be called "portable" because it was
        // intended as a way to get a slice view of `Bits` independent of `Digit` width.
        // It worked with unused bits by making the slice length such that the unused
        // bits were only in the last byte, which at first glance is portable. However,
        // I completely forgot about big-endian systems. Not taking a full width of the
        // byte slice can result in significant bytes being completely disregarded.
        let size_in_u8 = self.total_digits() * DIGIT_BYTES;
        // Safety: Adding on to what is satisfied in `as_slice`, [Digit] can always be
        // divided into [u8] and the correct length is calculated above. If the bitwidth
        // is not a multiple of eight, there must be at least enough unused bits to form
        // one more byte. This is returned as a reference with a constrained lifetime,
        // so we can't run into any deallocation alignment UB.
        unsafe { &*ptr::slice_from_raw_parts(self.as_ptr() as *const u8, size_in_u8) }
    }

    /// Returns a mutable reference to the underlying bits of `self`, including
    /// unused bits (which occur if `self.bw()` is not a multiple of
    /// `Digit::BITS`).
    ///
    /// # Note
    ///
    /// Unused bits can be temporarily set but should be cleared before they
    /// are used by another function that expects the standard `Bits` invariants
    /// to be upheld. Set unused bits will not cause Rust undefined behavior,
    /// but may cause incorrect arithmetical results or panics.
    ///
    /// # Portability
    ///
    /// This function is highly non-portable across architectures, see the
    /// source code of [Bits::rand_] for how to handle this
    #[doc(hidden)]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    #[inline(always)] // this is needed for `unstable_from_u8_slice`
    pub const fn as_mut_bytes_full_width_nonportable(&'a mut self) -> &'a mut [u8] {
        let size_in_u8 = self.total_digits() * DIGIT_BYTES;
        // Safety: Same reasoning as `as_bytes_full_width_nonportable`
        unsafe { &mut *ptr::slice_from_raw_parts_mut(self.as_mut_ptr() as *mut u8, size_in_u8) }
    }

    /// Assigns the bits of `buf` to `self`. If `(buf.len() * 8) > self.bw()`
    /// then the corresponding bits in `buf` beyond `self.bw()` are ignored. If
    /// `(buf.len() * 8) < self.bw()` then the rest of the bits in `self` are
    /// zeroed. This function is portable across target architecture pointer
    /// sizes and endianness.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn u8_slice_(&'a mut self, buf: &[u8]) {
        let self_byte_width = self.total_digits() * DIGIT_BYTES;
        let min_width = if self_byte_width < buf.len() {
            self_byte_width
        } else {
            buf.len()
        };
        // start of digits that will not be completely overwritten
        let start = min_width / DIGIT_BYTES;
        unsafe {
            // zero out first.
            self.digit_set(false, start..self.total_digits(), false);
            // Safety: `src` is valid for reads at least up to `min_width`, `dst` is valid
            // for writes at least up to `min_width`, they are aligned, and are
            // nonoverlapping because `self` is a mutable reference.
            ptr::copy_nonoverlapping(
                buf.as_ptr(),
                self.as_mut_bytes_full_width_nonportable().as_mut_ptr(),
                min_width,
            );
            // `start` can be `self.total_digits()`, so cap it
            let cap = if start >= self.total_digits() {
                self.total_digits()
            } else {
                start + 1
            };
            const_for!(i in {0..cap} {
                // correct for big endian, otherwise no-op
                *self.get_unchecked_mut(i) = Digit::from_le(self.get_unchecked(i));
            });
        }
        self.clear_unused_bits();
    }

    /// Assigns the bits of `self` to `buf`. If `(buf.len() * 8) > self.bw()`
    /// then the corresponding bits in `buf` beyond `self.bw()` are zeroed. If
    /// `(buf.len() * 8) < self.bw()` then the bits of `self` beyond the buffer
    /// do nothing. This function is portable across target architecture
    /// pointer sizes and endianness.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn to_u8_slice(&'a self, buf: &mut [u8]) {
        let self_byte_width = self.total_digits() * DIGIT_BYTES;
        let min_width = if self_byte_width < buf.len() {
            self_byte_width
        } else {
            buf.len()
        };
        #[cfg(target_endian = "little")]
        {
            unsafe {
                // Safety: `src` is valid for reads at least up to `min_width`, `dst` is valid
                // for writes at least up to `min_width`, they are aligned, and are
                // nonoverlapping because `buf` is a mutable reference.
                ptr::copy_nonoverlapping(
                    self.as_bytes_full_width_nonportable().as_ptr(),
                    buf.as_mut_ptr(),
                    min_width,
                );
            }
        }
        #[cfg(target_endian = "big")]
        {
            const_for!(i in {0..self.total_digits()} {
                let x = self.as_slice()[i];
                let start = i * DIGIT_BYTES;
                let end = if (start + DIGIT_BYTES) > buf.len() {
                    buf.len()
                } else {
                    start + DIGIT_BYTES
                };
                let mut s = 0;
                const_for!(j in {start..end} {
                    buf[j] = (x >> s) as u8;
                    s += 8;
                });
            });
        }
        unsafe {
            // zero remaining bytes.
            // Safety: `min_width` cannot be more than `buf.len()`
            ptr::write_bytes(buf.as_mut_ptr().add(min_width), 0, buf.len() - min_width);
        }
    }

    /// # Safety
    ///
    /// `range` must satisfy `range.start <= range.end` and `range.end <=
    /// self.total_digits()`
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const unsafe fn digit_set(
        &mut self,
        val: bool,
        range: Range<usize>,
        clear_unused_bits: bool,
    ) {
        debug_assert!(range.end <= self.total_digits());
        debug_assert!(range.start <= range.end);
        //let byte = if val { u8::MAX } else { 0 };
        //ptr::write_bytes(
        //    self.as_mut_ptr().add(range.start),
        //    byte,
        //    range.end - range.start,
        //);
        let digit = if val { MAX } else { 0 };
        unsafe_for_each_mut!(
            self,
            x,
            { range.start..range.end }
            { *x = digit },
            clear_unused_bits
        );
    }

    /// Gets one `Digit` from `self` starting at the bit index `start`.
    /// Bits that extend beyond `self.bw()` are zeroed.
    #[doc(hidden)]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn get_digit(&self, start: usize) -> Digit {
        let digits = digits_u(start);
        let bits = extra_u(start);
        let mut tmp = 0;
        // Safety: The checks avoid indexing beyond `self.total_digits() - 1`
        unsafe {
            if digits < self.total_digits() {
                tmp = self.get_unchecked(digits) >> bits;
                if bits != 0 && ((digits + 1) < self.total_digits()) {
                    tmp |= self.get_unchecked(digits + 1) << (BITS - bits);
                }
            }
            tmp
        }
    }

    /// Gets two `Digit`s from `self` starting at the bit index `start`,
    /// and returns them in little endian order. Bits that extend beyond
    /// `self.bw()` are zeroed.
    #[doc(hidden)]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn get_double_digit(&self, start: usize) -> (Digit, Digit) {
        let digits = digits_u(start);
        let bits = extra_u(start);
        let mut first = 0;
        let mut second = 0;
        // Safety: The checks avoid indexing beyond `self.total_digits() - 1`
        unsafe {
            if digits < self.total_digits() {
                first = self.get_unchecked(digits) >> bits;
                if (digits + 1) < self.total_digits() {
                    let mid = self.get_unchecked(digits + 1);
                    if bits == 0 {
                        second = mid;
                    } else {
                        first |= mid << (BITS - bits);
                        second = mid >> bits;
                        if (digits + 2) < self.total_digits() {
                            second |= self.get_unchecked(digits + 2) << (BITS - bits);
                        }
                    };
                }
            }
            (first, second)
        }
    }
}

impl fmt::Debug for Bits {
    /// Forwards to the `LowerHex` impl. We cannot use decimal because it would
    /// require allocation.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}

impl fmt::Display for Bits {
    /// Forwards to the `Debug` impl
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl fmt::LowerHex for Bits {
    /// Lowercase hexadecimal formatting.
    ///
    /// ```
    /// use awint::{inlawi, Bits, InlAwi};
    /// assert_eq!(
    ///     format!("{:x}", inlawi!(0xfedcba9876543210u100)),
    ///     "0xfedcba98_76543210_u100"
    /// );
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.debug_format_hexadecimal(f, false)
    }
}

impl fmt::UpperHex for Bits {
    /// Uppercase hexadecimal formatting.
    ///
    /// ```
    /// use awint::{inlawi, Bits, InlAwi};
    /// assert_eq!(
    ///     format!("{:X}", inlawi!(0xFEDCBA9876543210u100)),
    ///     "0xFEDCBA98_76543210_u100"
    /// );
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.debug_format_hexadecimal(f, true)
    }
}

impl fmt::Octal for Bits {
    /// Octal formatting.
    ///
    /// ```
    /// use awint::{inlawi, Bits, InlAwi};
    /// assert_eq!(
    ///     format!("{:o}", inlawi!(0o776543210u100)),
    ///     "0o7_76543210_u100"
    /// );
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.debug_format_octal(f)
    }
}

impl fmt::Binary for Bits {
    /// Binary formatting.
    ///
    /// ```
    /// use awint::{inlawi, Bits, InlAwi};
    /// assert_eq!(format!("{:b}", inlawi!(11000101)), "0b11000101_u8");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.debug_format_binary(f)
    }
}

impl fmt::Pointer for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr = self.as_ptr();
        fmt::Pointer::fmt(&ptr, f)
    }
}

impl Hash for Bits {
    /// note: this function is not portable across platforms
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bw().hash(state);
        self.as_slice().hash(state);
    }
}

#[cfg(feature = "zeroize_support")]
impl zeroize::Zeroize for Bits {
    fn zeroize(&mut self) {
        self.as_mut_slice().zeroize()
    }
}
