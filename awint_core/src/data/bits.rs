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

/// Because of the current limitations on custom DSTs, we had to resort to some
/// highly unsafe transmutes. To make the situation as safe as we could, we
/// shifted the transmute boundaries such that these transmutes only occur
/// between `&[usize], &Bits` and `&mut [usize], &mut Bits`. We also applied
/// `#[repr(transparent)]` to `Bits`. However, we still want to account for
/// unexpected changes by future compilers that are enough to break these. This
/// crate will fail to compile instead of causing accessible UB if this constant
/// fails.
const _ASSERT_BITS_ASSUMPTIONS: () = {
    let _ = ["Assertion that the size of `&mut [usize]` is what we expect"]
        [(mem::size_of::<&mut [usize]>() != mem::size_of::<&mut Bits>()) as usize];
    let _ = ["Assertion that the alignment of `&mut [usize]` is what we expect"]
        [(mem::align_of::<&mut [usize]>() != mem::align_of::<&mut Bits>()) as usize];

    // We cannot properly use `as *mut usize` in constants yet, but this should be
    // good enough. Make a round trip through two transmute boundaries to make sure
    // basic properties are not broken.
    let array: [usize; 2] = [42, 7]; // bitwidth of 7 and value 42
    let fat_ptr: *const usize = (&array) as *const usize;
    let bits: &Bits = unsafe { Bits::from_raw_parts(fat_ptr, array.len()) };
    let _ = ["Bitwidth check"][(bits.bw() != 7) as usize];
    let _ = ["Length check"][(bits.len() != 1) as usize];
    // these go through `get_unchecked` and `as_ptr` among other things
    let _ = ["Digit check"][(bits.first() != 42) as usize];
    let _ = ["Digit check"][(bits.last() != 42) as usize];
    let array_ref = bits.as_slice();
    let _ = ["Digit check"][(array_ref[0] != 42) as usize];
    let _ = ["Length check"][(array_ref.len() != 1) as usize];
};

/// A reference to the bits in an `InlAwi`, `ExtAwi`, or other backing
/// construct. This allows the same functions that operate on a dynamic `ExtAwi`
/// at runtime to also operate on an `InlAwi` at compile time.
///
/// `Bits` do **not** know signedness. Instead, the methods on `Bits` are
/// specified to interpret the bits as unsigned or signed two's complement
/// integers. If a method's documentation does not mention signedness, it either
/// works for both kinds or views the bits as a plain bit string with no
/// integral properties.
///
/// # Note
///
/// Unless otherwise specified, functions on `Bits` that return an `Option<()>`
/// return `None` if the input bitwidths are not equal to each other. The `Bits`
/// have been left unchanged if `None` is returned.
///
/// # Portability
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
/// needed). Most end users should not uses these, since they have
/// a strong dependence on the size of `usize`. These functions are actual views
/// into the inner building blocks of this crate that other functions are built
/// around in such a way that they are portable (e.g. the addition functions may
/// internally operate on differing numbers of `usize` digits depending on the
/// size of `usize`, but the end result looks the same to users on different
/// architectures). The only reason these functions are exposed, is that someone
/// may want to write their own custom performant algorithms, and they want as
/// few abstractions as possible in the way.
///
/// Visible functions that are not portable in general, but always start from
/// the zeroeth bit or a given bit position like [Bits::short_cin_mul],
/// [Bits::short_udivide_assign], or [Bits::usize_or_assign], are always
/// portable as long as the digit inputs and/or outputs are restricted to
/// `0..u16::MAX`.
///
/// The `Bits::as_bytes` function and related functions, the serialization impls
/// enabled by `serde_support`, the strings produced by the `const`
/// serialization functions, and the serialization free functions in the
/// `awint_ext` crate are all portable and should be used when sending
/// representations of `Bits` between architectures.
///
/// The `Hash` impl and the `rand_assign_using` function enabled by
/// `rand_support` use a deterministic byte oriented implementation to avoid
/// portability issues.
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
    pub const unsafe fn from_raw_parts(raw_ptr: *const usize, raw_len: usize) -> &'a Self {
        // Safety: `Bits` follows standard slice initialization invariants and is marked
        // `#[repr(transparent)]`. `_ASSERT_BITS_ASSUMPTIONS` also has safeguards. The
        // explicit lifetimes make sure they do not become unbounded.
        unsafe { mem::transmute::<&[usize], &Bits>(&*ptr::slice_from_raw_parts(raw_ptr, raw_len)) }
    }

    /// # Safety
    ///
    /// see [Bits::from_raw_parts]
    #[doc(hidden)]
    #[inline]
    pub const unsafe fn from_raw_parts_mut(raw_ptr: *mut usize, raw_len: usize) -> &'a mut Self {
        // Safety: `Bits` follows standard slice initialization invariants and is marked
        // `#[repr(transparent)]`. `_ASSERT_BITS_ASSUMPTIONS` also has safeguards. The
        // explicit lifetimes make sure they do not become unbounded.
        unsafe {
            mem::transmute::<&mut [usize], &mut Bits>(&mut *ptr::slice_from_raw_parts_mut(
                raw_ptr, raw_len,
            ))
        }
    }

    /// Returns a raw pointer to the underlying bit storage. The caller must
    /// ensure that the `Bits` outlives the pointer this function returns.
    /// The underlying memory should never be written to.
    #[doc(hidden)]
    #[inline]
    pub const fn as_ptr(&self) -> *const usize {
        self.raw.as_ptr()
    }

    /// Returns a raw pointer to the underlying bit storage. The caller must
    /// ensure that the `Bits` outlives the pointer this function returns.
    #[doc(hidden)]
    #[inline]
    pub const fn as_mut_ptr(&mut self) -> *mut usize {
        self.raw.as_mut_ptr()
    }

    /// Returns the raw total length of `self`, including the bitwidth digit.
    #[doc(hidden)]
    #[inline]
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
    pub(crate) const unsafe fn raw_get_unchecked_mut(&'a mut self, i: usize) -> &'a mut usize {
        debug_assert!(i < self.raw_len());
        // Safety: `i` is bounded by `raw_len`. The lifetimes are bounded.
        unsafe { &mut *self.as_mut_ptr().add(i) }
    }

    /// Returns the bitwidth as a `NonZeroUsize`
    #[inline]
    pub const fn nzbw(&self) -> NonZeroUsize {
        unsafe {
            // Safety: The bitwidth is stored in the last raw slice element. The bitwidth is
            // nonzero if invariants were maintained.
            let bw = self.raw_get_unchecked(self.raw_len() - 1);
            // If something with zeroing allocation or mutations accidentally breaks during
            // development, it will probably manifest itself here
            debug_assert!(bw != 0);
            NonZeroUsize::new_unchecked(bw)
        }
    }

    /// Returns the bitwidth as a `usize`
    #[inline]
    pub const fn bw(&self) -> usize {
        self.nzbw().get()
    }

    /// # Safety
    ///
    /// `i < self.len()` should hold true
    #[doc(hidden)]
    #[inline]
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
    pub const unsafe fn get_unchecked_mut(&'a mut self, i: usize) -> &'a mut usize {
        debug_assert!(i < self.len());
        // Safety: The bounds of this are a subset of `raw_get_unchecked_mut`
        unsafe { self.raw_get_unchecked_mut(i) }
    }

    /// Returns the exact number of `usize` digits needed to store all bits.
    #[inline]
    pub const fn len(&self) -> usize {
        // Safety: The length on the raw slice is the number of `usize` digits plus the
        // metadata bitwidth digit. To get the number of regular digits, we just
        // subtract the metadata digit.
        self.raw_len() - 1
    }

    /// Returns the number of unused bits.
    #[doc(hidden)]
    #[inline]
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
    pub const fn extra(&self) -> usize {
        extra(self.nzbw())
    }

    /// Returns the first `usize` digit
    #[doc(hidden)]
    #[inline]
    pub const fn first(&self) -> usize {
        // Safety: There is at least one digit since bitwidth has a nonzero invariant
        unsafe { self.get_unchecked(0) }
    }

    /// Returns a mutable reference to the first `usize` digit
    #[doc(hidden)]
    #[inline]
    pub const fn first_mut(&'a mut self) -> &'a mut usize {
        // Safety: There is at least one digit since bitwidth has a nonzero invariant
        unsafe { self.get_unchecked_mut(0) }
    }

    /// Returns the last `usize` digit
    #[doc(hidden)]
    #[inline]
    pub const fn last(&self) -> usize {
        // Safety: There is at least one digit since bitwidth has a nonzero invariant
        unsafe { self.get_unchecked(self.len() - 1) }
    }

    /// Returns a mutable reference to the last `usize` digit
    #[doc(hidden)]
    #[inline]
    pub const fn last_mut(&'a mut self) -> &'a mut usize {
        // Safety: There is at least one digit since bitwidth has a nonzero invariant
        unsafe { self.get_unchecked_mut(self.len() - 1) }
    }

    /// Clears the unused bits.
    #[inline]
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
    pub const fn as_slice(&'a self) -> &'a [usize] {
        // Safety: `Bits` already follows standard slice initialization invariants. This
        // acquires a subslice that includes everything except for the metadata digit.
        unsafe { &*ptr::slice_from_raw_parts(self.as_ptr(), self.len()) }
    }

    /// Same as [Bits::as_slice] except it includes the bitwidth digit
    #[doc(hidden)]
    #[inline]
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
    pub const fn as_mut_slice(&'a mut self) -> &'a mut [usize] {
        // Safety: `Bits` already follows standard slice initialization invariants. This
        // acquires a subslice that includes everything except for the metadata digit,
        // so that it cannot be mutated.
        unsafe { &mut *ptr::slice_from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
    }

    /// Returns a reference to the underlying bits of `self`, including unused
    /// bits (which occur if `self.bw()` is not a multiple of 8).
    ///
    /// # Note
    ///
    /// If the `Bits` has unused bits, those bits will always be set to zero,
    /// even if the `Bits` are intended to be a sign extended integer.
    pub const fn as_bytes(&'a self) -> &'a [u8] {
        // This will result in an 8 bit digit analog of unused bits
        let size_in_u8 = if (self.bw() % 8) == 0 {
            self.bw() / 8
        } else {
            (self.bw() / 8) + 1
        };
        // Safety: Adding on to what is satisfied in `as_slice`, [usize] can always be
        // divided into [u8] and the correct length is calculated above. If the bitwidth
        // is not a multiple of eight, there must be at least enough unused bits to form
        // one more byte.
        unsafe { &*ptr::slice_from_raw_parts(self.as_ptr() as *const u8, size_in_u8) }
    }

    /// Returns a mutable reference to the underlying bits of `self`, including
    /// unused bits (which occur if `self.bw()` is not a multiple of 8).
    ///
    /// # Note
    ///
    /// Unused bits can be temporarily set but should be cleared before they
    /// are used by another function that expects the standard `Bits` invariants
    /// to be upheld. Set unused bits will not cause Rust undefined behavior,
    /// but may cause incorrect arithmetical results or panics.
    pub const fn as_mut_bytes(&'a mut self) -> &'a mut [u8] {
        let size_in_u8 = if (self.bw() % 8) == 0 {
            self.bw() / 8
        } else {
            (self.bw() / 8) + 1
        };
        // Safety: Same reasoning as `as_bytes`
        unsafe { &mut *ptr::slice_from_raw_parts_mut(self.as_mut_ptr() as *mut u8, size_in_u8) }
    }

    /// # Safety
    ///
    /// `range` must satisfy `range.start <= range.end` and `range.end <=
    /// self.len()`
    #[doc(hidden)]
    #[inline]
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
        for_each_mut!(
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

/// Forwards to the `LowerHex` impl.
impl fmt::Debug for Bits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}

/// Lowercase hexadecimal formatting.
///
/// ```
/// use awint::{InlAwi, inlawi};
/// assert_eq!(format!("{:x}", inlawi!(0xfedcba9876543210u100)), "0xfedcba98_76543210_u100");
/// ```
impl fmt::LowerHex for Bits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.debug_format_hexadecimal(f, false)
    }
}

/// Uppercase hexadecimal formatting.
///
/// ```
/// use awint::{InlAwi, inlawi};
/// assert_eq!(format!("{:X}", inlawi!(0xFEDCBA9876543210u100)), "0xFEDCBA98_76543210_u100");
/// ```
impl fmt::UpperHex for Bits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.debug_format_hexadecimal(f, true)
    }
}

/// Octal formatting.
///
/// ```
/// use awint::{InlAwi, inlawi};
/// assert_eq!(format!("{:o}", inlawi!(0o776543210u100)), "0o7_76543210_u100");
/// ```
impl fmt::Octal for Bits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.debug_format_octal(f)
    }
}

/// Binary formatting.
///
/// ```
/// use awint::{inlawi, InlAwi};
/// assert_eq!(format!("{:b}", inlawi!(11000101)), "0b11000101_u8");
/// ```
impl fmt::Binary for Bits {
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
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bw().hash(state);
        self.as_bytes().hash(state);
    }
}
