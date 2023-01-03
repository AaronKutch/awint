//! ## Basic Invariants
//!
//! - `Bits` have a nonzero bit-width specified in a `NonZeroUsize`. Being
//!   nonzero, it eliminates several edge cases and ambiguities this crate would
//!   have to handle.
//! - `Bits` are stored in little endian order with the same requirements as
//!   `[usize]`. The number of `usize` digits is the minimum needed to store all
//!   bits. If bitwidth is not a multiple of `usize::BITS`, then there will be
//!   some unused bits in the last `usize` digit. For example, a bitwidth of of
//!   100 bits takes up 2 digits (if `usize::BITS == 64`): 64 bits in the first
//!   digit, 36 bits in the least significant bits of the second, and 28 unused
//!   bits in the remaining bits of the second.
//! - Unused bits are zeroed. Note that this is not a safety critical invariant.
//!   Setting unused bits via `Bits::as_mut_slice` or `Bits::last_mut` will not
//!   cause Rust U.B., but it may result in arithmetically incorrect results or
//!   panics from the functions on `Bits`. Arbitrary bits can in the last digit
//!   can be set temporarily, but [Bits::clear_unused_bits] should be run before
//!   reaching a function that expects all these invariants to hold.

use core::{
    fmt,
    hash::{Hash, Hasher},
    mem,
    num::NonZeroUsize,
    ops::Range,
    ptr,
};

use awint_internals::*;
use const_fn::const_fn;

const BYTE_RATIO: usize = (usize::BITS / u8::BITS) as usize;

/// A reference to the bits in an `InlAwi`, `ExtAwi`, or other backing
/// construct. If a function is written just in terms of `Bits`, it can work on
/// mixed references to `InlAwi`s, `ExtAwi`s, and `FP<B>`s.
/// `const` big integer arithmetic is possible if the backing type is `InlAwi`
/// and the "const_support" flag is enabled.
///
/// `Bits` do **not** know signedness. Instead, the methods on `Bits` are
/// specified to interpret the bits as unsigned or signed two's complement
/// integers. If a method's documentation does not mention signedness, it either
/// works for both kinds or views the bits as a plain bit string with no
/// integral properties.
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
/// with different `usize::BITS` and different endiannesses. The
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
/// There are many functions that depend on `usize` and `NonZeroUsize`. In cases
/// where the `usize` describes the bitwidth, a bit shift, or a bit position,
/// the user should not need to worry about portability, since if the values are
/// close to `usize::MAX`, the user is already close to running out of possible
/// memory any way.
///
/// There are a few usages of `usize` that are not just indexes but are actual
/// views into a contiguous range of bits inside `Bits`, such as
/// `Bits::as_slice`, `Bits::first`, and `Bits::get_digit` (which are all hidden
/// from the documentation, please refer to the source code of `bits.rs` if
/// needed). Most end users should not use these, since they have a strong
/// dependence on the size of `usize`. These functions are actual views into the
/// inner building blocks of this crate that other functions are built around in
/// such a way that they are portable (e.g. the addition functions may
/// internally operate on differing numbers of `usize` digits depending on the
/// size of `usize`, but the end result looks the same to users on different
/// architectures). The only reason these functions are exposed, is that someone
/// may want to write their own custom performant algorithms, and they want as
/// few abstractions as possible in the way.
///
/// Visible functions that are not portable in general, but always start from
/// the zeroeth bit or a given bit position like [Bits::short_cin_mul],
/// [Bits::short_udivide_], or [Bits::usize_or_], are always
/// portable as long as the digit inputs and/or outputs are restricted to
/// `0..=u16::MAX`, or special care is taken.
#[repr(transparent)]
pub struct Bits {
    /// # Raw Invariants
    ///
    /// We have chosen `Bits` to be a DST in order to avoid double indirection
    /// (`&mut Bits` would be a pointer to a `Bits` struct which in turn had a
    /// pointer inside itself to the actual digits). A DST also lets us harness
    /// the power of Rust's desugering and inference surrounding other DSTS.
    ///
    /// In addition to the minimum number of digits required to store all the
    /// bits, there is one more metadata digit on the end of the slice
    /// responsible for storing the actual bitwidth. The length field on the
    /// `[usize]` DST is the total number of digits in the slice, including
    /// regular digits and the metadata digit. This design decision was made to
    /// prevent invoking UB by having a fake slice with the bitwidth instead of
    /// the true slice width. Even if we completely avoid all Rust core methods
    /// on slices (and thus avoid practical UB due to avoiding standard slice
    /// functions expecting a standard length field), Miri can still detect a
    /// fake slice being made (even if we completely avoid
    /// `core::ptr::slice_from_raw_parts`).
    ///
    /// The metadata bitwidth is on the end of the slice, because accesses of
    /// the bitwidth also commonly access the last digit right next to it
    /// through `clear_unused_bits`. This means good cache locality even if the
    /// slice is huge and interior digits are rarely accessed. Storing the
    /// bitwidth at the beginning of the slice instead (which is what Rust does
    /// if we add the bitwidth directly as a field in the `Bits` DST) would lead
    /// to extra offsetting operations being done to skip the first digit
    /// pointed to by the pointer in the DST.
    ///
    /// The unfortunate consequence is that taking `Bits` digitwise subslices of
    /// `Bits` in the same general no-copy way that you can take subslices of
    /// regular Rust slices is not possible. `subdigits_mut!` almost achieves it
    /// by temporarily replacing a digit with the needed metadata where the end
    /// of the subslice is, running a closure on the subslice, and
    /// then replacing the digit at the end. A different crate without fine
    /// bitwidth control would have to be spun off of this one.
    raw: [usize],
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
    /// `raw_ptr` and `raw_len` should satisfy the raw invariants (not just the
    /// regular invariants, but those that account for the extra bitwidth digit)
    /// of `Bits` along with [standard alignment and initialization
    /// conditions](core::slice::from_raw_parts_mut). `Bits` itself does not
    /// allocate or deallocate memory. It is expected that the caller had a
    /// struct with proper `Drop` implementation, created `Bits` from that
    /// struct, and insured that the struct is borrowed for the duration of
    /// the `Bits` lifetime. The memory referenced by `bits` must not be
    /// accessed through any other reference for the duration of lifetime `'a`.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const unsafe fn from_raw_parts(raw_ptr: *const usize, raw_len: usize) -> &'a Self {
        // Safety: `Bits` follows standard slice initialization invariants and is marked
        // `#[repr(transparent)]`. The explicit lifetimes make sure they do not become
        // unbounded.
        unsafe { mem::transmute::<&[usize], &Bits>(&*ptr::slice_from_raw_parts(raw_ptr, raw_len)) }
    }

    /// # Safety
    ///
    /// see [Bits::from_raw_parts]
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const unsafe fn from_raw_parts_mut(raw_ptr: *mut usize, raw_len: usize) -> &'a mut Self {
        // Safety: `Bits` follows standard slice initialization invariants and is marked
        // `#[repr(transparent)]`. The explicit lifetimes make sure they do not become
        // unbounded.
        unsafe {
            mem::transmute::<&mut [usize], &mut Bits>(&mut *ptr::slice_from_raw_parts_mut(
                raw_ptr, raw_len,
            ))
        }
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
    pub const fn as_ptr(&self) -> *const usize {
        self.raw.as_ptr()
    }

    /// Returns a raw pointer to the underlying bit storage. The caller must
    /// ensure that the `Bits` outlives the pointer this function returns.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn as_mut_ptr(&mut self) -> *mut usize {
        self.raw.as_mut_ptr()
    }

    /// Returns the raw total length of `self`, including the bitwidth digit.
    #[doc(hidden)]
    #[inline]
    #[must_use]
    pub const fn raw_len(&self) -> usize {
        self.raw.len()
    }

    /// This allows access of all digits including the bitwidth digit.
    ///
    /// # Safety
    ///
    /// `i < self.raw_len()` should hold true
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub(crate) const unsafe fn raw_get_unchecked(&self, i: usize) -> usize {
        debug_assert!(i < self.raw_len());
        // Safety: `i` is bounded by `raw_len`
        unsafe { *self.as_ptr().add(i) }
    }

    /// This allows mutable access of all digits including the bitwidth digit.
    ///
    /// # Safety
    ///
    /// `i < self.raw_len()` should hold true
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub(crate) const unsafe fn raw_get_unchecked_mut(&'a mut self, i: usize) -> &'a mut usize {
        debug_assert!(i < self.raw_len());
        // Safety: `i` is bounded by `raw_len`. The lifetimes are bounded.
        unsafe { &mut *self.as_mut_ptr().add(i) }
    }

    /// Returns the bitwidth as a `NonZeroUsize`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn nzbw(&self) -> NonZeroUsize {
        unsafe {
            // Safety: The bitwidth is stored in the last raw slice element. The bitwidth is
            // nonzero if invariants were maintained.
            let w = self.raw_get_unchecked(self.raw_len() - 1);
            // If something with zeroing allocation or mutations accidentally breaks during
            // development, it will probably manifest itself here
            debug_assert!(w != 0);
            NonZeroUsize::new_unchecked(w)
        }
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
    /// `i < self.len()` should hold true
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const unsafe fn get_unchecked(&self, i: usize) -> usize {
        debug_assert!(i < self.len());
        // Safety: `i < self.len()` means the access is within the slice
        unsafe { self.raw_get_unchecked(i) }
    }

    /// # Safety
    ///
    /// `i < self.len()` should hold true
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const unsafe fn get_unchecked_mut(&'a mut self, i: usize) -> &'a mut usize {
        debug_assert!(i < self.len());
        // Safety: The bounds of this are a subset of `raw_get_unchecked_mut`
        unsafe { self.raw_get_unchecked_mut(i) }
    }

    /// Returns the exact number of `usize` digits needed to store all bits.
    #[doc(hidden)]
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        // Safety: The length on the raw slice is the number of `usize` digits plus the
        // metadata bitwidth digit. To get the number of regular digits, we just
        // subtract the metadata digit.
        self.raw_len() - 1
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

    /// Returns the number of extra bits, or `usize::BITS - self.unused()`. If
    /// there are no unused bits, this is zero.
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn extra(&self) -> usize {
        extra(self.nzbw())
    }

    /// Returns the first `usize` digit
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn first(&self) -> usize {
        // Safety: There is at least one digit since bitwidth has a nonzero invariant
        unsafe { self.get_unchecked(0) }
    }

    /// Returns a mutable reference to the first `usize` digit
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn first_mut(&'a mut self) -> &'a mut usize {
        // Safety: There is at least one digit since bitwidth has a nonzero invariant
        unsafe { self.get_unchecked_mut(0) }
    }

    /// Returns the last `usize` digit
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn last(&self) -> usize {
        // Safety: There is at least one digit since bitwidth has a nonzero invariant
        unsafe { self.get_unchecked(self.len() - 1) }
    }

    /// Returns a mutable reference to the last `usize` digit
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn last_mut(&'a mut self) -> &'a mut usize {
        // Safety: There is at least one digit since bitwidth has a nonzero invariant
        unsafe { self.get_unchecked_mut(self.len() - 1) }
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
        if (self.extra() != 0) && (self.last() >= 1usize.wrapping_shl(self.extra() as u32)) {
            panic!("unused bits are set");
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
    pub const fn as_slice(&'a self) -> &'a [usize] {
        // Safety: `Bits` already follows standard slice initialization invariants. This
        // acquires a subslice that includes everything except for the metadata digit.
        unsafe { &*ptr::slice_from_raw_parts(self.as_ptr(), self.len()) }
    }

    /// Same as [Bits::as_slice] except it includes the bitwidth digit
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn as_raw_slice(&'a self) -> &'a [usize] {
        // Safety: `Bits` already follows standard slice initialization invariants. This
        // acquires a subslice that includes everything except for the metadata digit.
        unsafe { &*ptr::slice_from_raw_parts(self.as_ptr(), self.raw_len()) }
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
    pub const fn as_mut_slice(&'a mut self) -> &'a mut [usize] {
        // Safety: `Bits` already follows standard slice initialization invariants. This
        // acquires a subslice that includes everything except for the metadata digit,
        // so that it cannot be mutated.
        unsafe { &mut *ptr::slice_from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
    }

    /// Returns a reference to the underlying bits of `self`, including unused
    /// bits (which occur if `self.bw()` is not a multiple of `usize::BITS`).
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
        // intended as a way to get a slice view of `Bits` independent of `usize` width.
        // It worked with unused bits by making the slice length such that the unused
        // bits were only in the last byte, which at first glance is portable. However,
        // I completely forgot about big-endian systems. Not taking a full width of the
        // byte slice can result in significant bytes being completely disregarded.
        let size_in_u8 = self.len() * BYTE_RATIO;
        // Safety: Adding on to what is satisfied in `as_slice`, [usize] can always be
        // divided into [u8] and the correct length is calculated above. If the bitwidth
        // is not a multiple of eight, there must be at least enough unused bits to form
        // one more byte. This is returned as a reference with a constrained lifetime,
        // so we can't run into any deallocation alignment UB.
        unsafe { &*ptr::slice_from_raw_parts(self.as_ptr() as *const u8, size_in_u8) }
    }

    /// Returns a mutable reference to the underlying bits of `self`, including
    /// unused bits (which occur if `self.bw()` is not a multiple of
    /// `usize::BITS`).
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
    pub const fn as_mut_bytes_full_width_nonportable(&'a mut self) -> &'a mut [u8] {
        let size_in_u8 = self.len() * BYTE_RATIO;
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
        let self_byte_width = self.len() * BYTE_RATIO;
        let min_width = if self_byte_width < buf.len() {
            self_byte_width
        } else {
            buf.len()
        };
        // start of digits that will not be completely overwritten
        let start = min_width / BYTE_RATIO;
        unsafe {
            // zero out first.
            self.digit_set(false, start..self.len(), false);
            // Safety: `src` is valid for reads at least up to `min_width`, `dst` is valid
            // for writes at least up to `min_width`, they are aligned, and are
            // nonoverlapping because `self` is a mutable reference.
            ptr::copy_nonoverlapping(
                buf.as_ptr(),
                self.as_mut_bytes_full_width_nonportable().as_mut_ptr(),
                min_width,
            );
            // `start` can be `self.len()`, so cap it
            let cap = if start >= self.len() {
                self.len()
            } else {
                start + 1
            };
            const_for!(i in {0..cap} {
                // correct for big endian, otherwise no-op
                *self.get_unchecked_mut(i) = usize::from_le(self.get_unchecked(i));
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
        let self_byte_width = self.len() * BYTE_RATIO;
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
            const_for!(i in {0..self.len()} {
                let x = self.as_slice()[i];
                let start = i * BYTE_RATIO;
                let end = if (start + BYTE_RATIO) > buf.len() {buf.len()} else {start + BYTE_RATIO};
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
    /// self.len()`
    #[doc(hidden)]
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub const unsafe fn digit_set(
        &mut self,
        val: bool,
        range: Range<usize>,
        clear_unused_bits: bool,
    ) {
        debug_assert!(range.end <= self.len());
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

    /// Gets one `usize` digit from `self` starting at the bit index `start`.
    /// Bits that extend beyond `self.bw()` are zeroed.
    #[doc(hidden)]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn get_digit(&self, start: usize) -> usize {
        let digits = digits_u(start);
        let bits = extra_u(start);
        let mut tmp = 0;
        // Safety: The checks avoid indexing beyond `self.len() - 1`
        unsafe {
            if digits < self.len() {
                tmp = self.get_unchecked(digits) >> bits;
                if bits != 0 && ((digits + 1) < self.len()) {
                    tmp |= self.get_unchecked(digits + 1) << (BITS - bits);
                }
            }
            tmp
        }
    }

    /// Gets two `usize` digits from `self` starting at the bit index `start`,
    /// and returns them in little endian order. Bits that extend beyond
    /// `self.bw()` are zeroed.
    #[doc(hidden)]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn get_double_digit(&self, start: usize) -> (usize, usize) {
        let digits = digits_u(start);
        let bits = extra_u(start);
        let mut first = 0;
        let mut second = 0;
        // Safety: The checks avoid indexing beyond `self.len() - 1`
        unsafe {
            if digits < self.len() {
                first = self.get_unchecked(digits) >> bits;
                if (digits + 1) < self.len() {
                    let mid = self.get_unchecked(digits + 1);
                    if bits == 0 {
                        second = mid;
                    } else {
                        first |= mid << (BITS - bits);
                        second = mid >> bits;
                        if (digits + 2) < self.len() {
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
