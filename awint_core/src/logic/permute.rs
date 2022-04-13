use core::{mem::MaybeUninit, num::NonZeroUsize, ptr};

use awint_internals::*;
use const_fn::const_fn;

use crate::Bits;

/// Rotates a slice range so that `mid` is at element 0.
///
/// Rust's slice rotation function was highly optimized by me, but I had to copy
/// it from https://github.com/rust-lang/rust/blob/master/library/core/src/slice/mod.rs
/// and specialize it to work as `const`.
///
/// # Safety
///
/// The range `[mid-left, mid+right)` must be valid for reading and writing
#[const_fn(cfg(feature = "const_support"))]
const unsafe fn usize_rotate(mut left: usize, mut mid: *mut usize, mut right: usize) {
    unsafe {
        type BufType = [usize; 32];
        loop {
            if (right == 0) || (left == 0) {
                return
            }
            if left + right < 24 {
                // Algorithm 1
                let x = mid.sub(left);
                let mut tmp: usize = x.read();
                let mut i = right;
                let mut gcd = right;
                loop {
                    let tmp_tmp = x.add(i).read();
                    x.add(i).write(tmp);
                    tmp = tmp_tmp;
                    if i >= left {
                        i -= left;
                        if i == 0 {
                            x.write(tmp);
                            break
                        }
                        if i < gcd {
                            gcd = i;
                        }
                    } else {
                        i += right;
                    }
                }
                const_for!(start in {1..gcd} {
                    tmp = x.add(start).read();
                    i = start + right;
                    loop {
                        let tmp_tmp = x.add(i).read();
                        x.add(i).write(tmp);
                        tmp = tmp_tmp;
                        if i >= left {
                            i -= left;
                            if i == start {
                                x.add(start).write(tmp);
                                break;
                            }
                        } else {
                            i += right;
                        }
                    }
                });
                return
            // I have tested this with Miri to make sure it doesn't complain
            } else if left <= 32 || right <= 32 {
                // Algorithm 2
                let mut rawarray = MaybeUninit::<(BufType, [usize; 0])>::uninit();
                let buf = rawarray.as_mut_ptr() as *mut usize;
                let dim = mid.sub(left).add(right);
                if left <= right {
                    ptr::copy_nonoverlapping(mid.sub(left), buf, left);
                    ptr::copy(mid, mid.sub(left), right);
                    ptr::copy_nonoverlapping(buf, dim, left);
                } else {
                    ptr::copy_nonoverlapping(mid, buf, right);
                    ptr::copy(mid.sub(left), dim, left);
                    ptr::copy_nonoverlapping(buf, mid.sub(left), right);
                }
                return
            } else if left >= right {
                // Algorithm 3
                loop {
                    ptr::swap_nonoverlapping(mid.sub(right), mid, right);
                    mid = mid.sub(right);
                    left -= right;
                    if left < right {
                        break
                    }
                }
            } else {
                // Algorithm 3, `left < right`
                loop {
                    ptr::swap_nonoverlapping(mid.sub(left), mid, left);
                    mid = mid.add(left);
                    right -= left;
                    if right < left {
                        break
                    }
                }
            }
        }
    }
}

// `usize_rotate` has some unusually large thresholds for some branches that
// don't get tested well by Miri in fuzz.rs, so test them here
#[test]
fn usize_rotate_test() {
    let mut buf = [123usize; 123];
    for k in 0..123 {
        unsafe { usize_rotate(k, buf.as_mut_ptr().add(k), 123 - k) }
    }
}

/// # Bit permutation
impl Bits {
    /// Shift-left-assigns at the digit level
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub(crate) const fn digit_shl_assign(&mut self, s: NonZeroUsize) {
        // Should get optimized away when this function is inlined
        assert!(s.get() < self.bw());
        let s = digits(s);
        if s == 0 {
            return
        }
        unsafe {
            // below performs this:
            //const_for!(i in {s..self.len()}.rev() {
            //    *self.get_unchecked_mut(i) = self.get_unchecked(i - s);
            //});
            // If this shift overflows, it does not result in UB because it falls back to
            // the more relaxed `ptr:copy`
            if s.wrapping_shl(1) >= self.len() {
                // We cannot call multiple `as_ptr` or `as_mut_ptr` at the same time, because
                // they come from the same allocation. One would invalidate the other, and we
                // would run into stacked borrow issues.
                let ptr = self.as_mut_ptr();
                ptr::copy_nonoverlapping(ptr, ptr.add(s), self.len() - s);
            } else {
                let ptr = self.as_mut_ptr();
                ptr::copy(ptr, ptr.add(s), self.len() - s);
            }
            self.digit_set(false, 0..s, false);
        }
    }

    /// Shift-left-assigns according to extra bits
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub(crate) const fn subdigit_shl_assign(&mut self, s: NonZeroUsize, clear_unused_bits: bool) {
        let s = extra(s);
        if s != 0 {
            // TODO benchmark this strategy vs dual unroll
            const_for!(i in {1..self.len()}.rev() {
                unsafe {
                    *self.get_unchecked_mut(i) =
                        (self.get_unchecked(i - 1) >> (BITS - s)) | (self.get_unchecked(i) << s);
                }
            });
            *self.first_mut() <<= s;
        }
        if clear_unused_bits {
            self.clear_unused_bits();
        }
    }

    /// Shift-right-assigns at the digit level
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub(crate) const fn digit_shr_assign(
        &mut self,
        s: NonZeroUsize,
        extension: bool,
        clear_unused_bits: bool,
    ) {
        assert!(s.get() < self.bw());
        let s = digits(s);
        if s == 0 {
            if clear_unused_bits {
                self.clear_unused_bits();
            }
            return
        }
        unsafe {
            // below performs this:
            //const_for!(i in {s..self.len()} {
            //    *self.get_unchecked_mut(i - s) = self.get_unchecked(i);
            //});
            // If this shift overflows, it does not result in UB because it falls back to
            // the more relaxed `ptr:copy`
            if s.wrapping_shl(1) >= self.len() {
                // We cannot call multiple `as_ptr` or `as_mut_ptr` at the same time, because
                // they come from the same allocation. One would invalidate the other, and we
                // would run into stacked borrow issues.
                let ptr = self.as_mut_ptr();
                ptr::copy_nonoverlapping(ptr.add(s), ptr, self.len() - s);
            } else {
                let ptr = self.as_mut_ptr();
                ptr::copy(ptr.add(s), ptr, self.len() - s);
            }
            if extension && (self.unused() != 0) {
                // Safety: There are fewer digit shifts than digits
                *self.get_unchecked_mut(self.len() - 1 - s) |= MAX << (BITS - self.unused());
            }
            self.digit_set(extension, (self.len() - s)..self.len(), clear_unused_bits);
        }
    }

    /// Shift-right-assigns according to extra bits
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    pub(crate) const fn subdigit_shr_assign(
        &mut self,
        s: NonZeroUsize,
        extension: bool,
        clear_unused_bits: bool,
    ) {
        let s = extra(s);
        if s == 0 {
            if clear_unused_bits {
                self.clear_unused_bits();
            }
            return
        }
        unsafe {
            // TODO benchmark this strategy vs dual unroll
            const_for!(i in {0..(self.len() - 1)} {
                *self.get_unchecked_mut(i) =
                    (self.get_unchecked(i) >> s) | (self.get_unchecked(i + 1) << (BITS - s));
            });
            *self.last_mut() >>= s;
            if extension {
                if (s + self.unused()) > BITS {
                    *self.last_mut() = MAX;
                    // handle bits that get shifted into the next to last digit
                    // Safety: it is not possible to reach this unless there are enough bits for the
                    // shift which has to be less than the bitwidth
                    *self.get_unchecked_mut(self.len() - 2) |=
                        MAX << ((2 * BITS) - s - self.unused());
                } else {
                    *self.last_mut() |= MAX << (BITS - s - self.unused());
                }
            }
            if clear_unused_bits {
                self.clear_unused_bits();
            }
        }
    }

    /// Left-shifts-assigns by `s` bits. If `s >= self.bw()`, then
    /// `None` is returned and the `Bits` are left unchanged.
    ///
    /// Left shifts can act as a very fast multiplication by a power of two for
    /// both the signed and unsigned interpretation of `Bits`.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn shl_assign(&mut self, s: usize) -> Option<()> {
        match NonZeroUsize::new(s) {
            None => Some(()),
            Some(s) if s.get() < self.bw() => {
                self.digit_shl_assign(s);
                self.subdigit_shl_assign(s, true);
                Some(())
            }
            _ => None,
        }
    }

    /// Logically-right-shift-assigns by `s` bits. If `s >= self.bw()`, then
    /// `None` is returned and the `Bits` are left unchanged.
    ///
    /// Logical right shifts do not copy the sign bit, and thus can act as a
    /// very fast floored division by a power of two for the unsigned
    /// interpretation of `Bits`.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn lshr_assign(&mut self, s: usize) -> Option<()> {
        match NonZeroUsize::new(s) {
            None => Some(()),
            Some(s) if s.get() < self.bw() => {
                self.digit_shr_assign(s, false, false);
                self.subdigit_shr_assign(s, false, true);
                Some(())
            }
            _ => None,
        }
    }

    /// Arithmetically-right-shift-assigns by `s` bits. If `s >= self.bw()`,
    /// then `None` is returned and the `Bits` are left unchanged.
    ///
    /// Arithmetic right shifts copy the sign bit, and thus can act as a very
    /// fast _floored_ division by a power of two for the signed interpretation
    /// of `Bits`.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn ashr_assign(&mut self, s: usize) -> Option<()> {
        match NonZeroUsize::new(s) {
            None => Some(()),
            Some(s) if s.get() < self.bw() => {
                let extension = self.msb();
                self.digit_shr_assign(s, extension, false);
                self.subdigit_shr_assign(s, extension, true);
                Some(())
            }
            _ => None,
        }
    }

    /// Left-rotate-assigns by `s` bits. If `s >= self.bw()`, then
    /// `None` is returned and the `Bits` are left unchanged.
    ///
    /// This function is equivalent to the following:
    /// ```
    /// use awint::{extawi, inlawi, Bits, ExtAwi, InlAwi};
    /// let mut input = inlawi!(0x4321u16);
    /// let mut output = inlawi!(0u16);
    /// // rotate left by 4 bits or one hexadecimal digit
    /// let shift = 4;
    ///
    /// output.copy_assign(&input).unwrap();
    /// // temporary clone of the input
    /// let mut tmp = ExtAwi::from(input);
    /// if shift != 0 {
    ///     if shift >= input.bw() {
    ///         panic!();
    ///     }
    ///     output.shl_assign(shift).unwrap();
    ///     tmp.lshr_assign(input.bw() - shift).unwrap();
    ///     output.or_assign(&tmp);
    /// };
    ///
    /// assert_eq!(output, inlawi!(0x3214u16));
    /// let mut using_rotate = ExtAwi::from(input);
    /// using_rotate.rotl_assign(shift).unwrap();
    /// assert_eq!(using_rotate, extawi!(0x3214u16));
    ///
    /// // Note that slices are typed in a little-endian order opposite of
    /// // how integers are typed, but they still visually rotate in the
    /// // same way. This means `Rust`s built in slice rotation is in the
    /// // opposite direction to integers and `Bits`
    /// let mut array = [4, 3, 2, 1];
    /// array.rotate_left(1);
    /// assert_eq!(array, [3, 2, 1, 4]);
    /// assert_eq!(0x4321u16.rotate_left(4), 0x3214);
    /// let mut x = inlawi!(0x4321u16);
    /// x.rotl_assign(4);
    /// // `Bits` has the preferred endianness
    /// assert_eq!(x, inlawi!(0x3214u16));
    /// ```
    ///
    /// Unlike the example above which needs cloning, this function avoids any
    /// allocation and has many optimized branches for different input sizes and
    /// shifts.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn rotl_assign(&mut self, s: usize) -> Option<()> {
        match NonZeroUsize::new(s) {
            None => Some(()),
            Some(s) if s.get() < self.bw() => {
                let x = self;
                // fast path and simplifies other code paths
                if x.len() == 1 {
                    *x.last_mut() = ((x.last() >> (x.bw() - s.get())) | (x.last() << s.get()))
                        & (MAX >> (BITS - x.bw()));
                    return Some(())
                }
                // TODO implement faster `subdigit_rotate_right` branch for certain cases

                let digits = digits(s);
                // Note: this is not a bitwidth but a shift
                let s0 = extra(s);
                let extra = x.extra();

                let mid_digit = x.len() - digits;
                let p = x.as_mut_ptr();
                // Safety: this satisfies the requirements of `usize_rotate`
                unsafe {
                    usize_rotate(mid_digit, p.add(mid_digit), digits);
                }

                if extra != 0 && digits != 0 {
                    // fix unused bits left in the middle from the rotation
                    let wrap = x.last() >> extra;
                    unsafe {
                        subdigits_mut!(x, 0..digits, y, {
                            y.subdigit_shl_assign(NonZeroUsize::new_unchecked(BITS - extra), false)
                        });
                    }
                    *x.first_mut() |= wrap;
                    x.clear_unused_bits();
                }
                if s0 != 0 {
                    // apply subdigit rotation
                    let wrap = if extra == 0 {
                        x.last() >> (BITS - s0)
                    } else if s0 <= extra {
                        x.last() >> (extra - s0)
                    } else {
                        // bits from the second to last digit get rotated all the way through the
                        // extra bits. We have already handled `x.len() == 1`.
                        unsafe {
                            (x.last() << (s0 - extra))
                                | (x.get_unchecked(x.len() - 2) >> (BITS - s0 + extra))
                        }
                    };
                    x.subdigit_shl_assign(s, true);
                    *x.first_mut() |= wrap;
                }

                Some(())
            }
            _ => None,
        }
    }

    /// Right-rotate-assigns by `s` bits. If `s >= self.bw()`, then
    /// `None` is returned and the `Bits` are left unchanged.
    ///
    /// See `Bits::rotl_assign` for more details.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn rotr_assign(&mut self, s: usize) -> Option<()> {
        let bw = self.bw();
        if s == 0 {
            return Some(())
        } else if s >= bw {
            return None
        }
        self.rotl_assign(bw - s)
    }

    /// Reverse-bit-order-assigns `self`. The least significant bit becomes the
    /// most significant bit, the second least significant bit becomes the
    /// second most significant bit, etc.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn rev_assign(&mut self) {
        let len = self.len();
        if len == 1 {
            *self.last_mut() = self.last().reverse_bits() >> self.unused();
            return
        }
        let halfway = len >> 1;
        let odd = (len & 1) != 0;
        if self.extra() == 0 {
            unsafe {
                const_for!(i in {0..halfway} {
                    // swap opposite reversed digits until reaching the halfway point
                    let tmp = self.get_unchecked(i).reverse_bits();
                    *self.get_unchecked_mut(i) = self.get_unchecked_mut(len - 1 - i).reverse_bits();
                    *self.get_unchecked_mut(len - 1 - i) = tmp;
                });
                if odd {
                    // reverse the digit in the middle inplace
                    let tmp = self.get_unchecked_mut(halfway);
                    *tmp = tmp.reverse_bits();
                }
            }
        } else {
            if len == 2 {
                let tmp0 = self.first().reverse_bits();
                let tmp1 = self.last().reverse_bits();
                *self.first_mut() = tmp1 >> self.unused() | tmp0 << self.extra();
                *self.last_mut() = tmp0 >> self.unused();
                return
            }
            unsafe {
                let unused = self.unused();
                let extra = self.extra();
                // There are four temporaries, two starting in the least significant digits and
                // two starting in the most significant digits. `tmp0` starts initialized with
                // zero, so that the new unused bits are set to zero. If there are 7 digits,
                // then the temporary assignments look like:
                //
                // tmp0=0 | .... | .... | .... | .... | .... | .... | tmp3 |
                // tmp0=0 | tmp1 | .... | .... | .... | .... | tmp2 | tmp3 |
                //        | done | tmp0 | .... | .... | .... | tmp2 | done |
                //        | done | tmp0 | tmp1 | .... | tmp3 | tmp2 | done |
                //        | done | done | tmp1 | .... | tmp3 | done | done |
                //        | done | done | tmp1 |tmp0&2| tmp3 | done | done |
                //        | done | done | done |tmp0&2| done | done | done |
                //        | done | done | done |bridge| done | done | done |
                let mut i0 = 0;
                let mut tmp0 = 0;
                let mut tmp1;
                let mut i2 = len - 1;
                let mut tmp2;
                let mut tmp3 = self.get_unchecked(i2).reverse_bits();
                loop {
                    if i0 == halfway {
                        // bridge between the converging indexes
                        if odd {
                            *self.get_unchecked_mut(i2) = (tmp3 >> unused) | (tmp0 << extra);
                        }
                        break
                    }
                    tmp1 = self.get_unchecked(i0).reverse_bits();
                    tmp2 = self.get_unchecked(i2 - 1).reverse_bits();
                    *self.get_unchecked_mut(i0) = (tmp3 >> unused) | (tmp2 << extra);
                    *self.get_unchecked_mut(i2) = (tmp1 >> unused) | (tmp0 << extra);
                    i0 += 1;
                    i2 -= 1;
                    if i0 == halfway {
                        if odd {
                            *self.get_unchecked_mut(i0) = (tmp2 >> unused) | (tmp1 << extra);
                        }
                        break
                    }
                    tmp0 = self.get_unchecked(i0).reverse_bits();
                    tmp3 = self.get_unchecked(i2 - 1).reverse_bits();
                    *self.get_unchecked_mut(i0) = (tmp2 >> unused) | (tmp3 << extra);
                    *self.get_unchecked_mut(i2) = (tmp0 >> unused) | (tmp1 << extra);
                    i0 += 1;
                    i2 -= 1;
                }
            }
        }
    }

    /// Funnel shift with power-of-two bitwidths. Returns `None` if
    /// `2*self.bw() != rhs.bw() || 2^s.bw() != self.bw()`. A `self.bw()` sized
    /// field is assigned to `self` from `rhs` starting from the bit position
    /// `s`. The shift cannot overflow because of the restriction on the
    /// bitwidth of `s`.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn funnel(&mut self, rhs: &Self, s: &Self) -> Option<()> {
        // because we later call `s.to_usize()` and assume it fits within `s.bw()`
        s.assert_cleared_unused_bits();
        // We avoid overflow by checking in this order and with `BITS - 1` instead of
        // `BITS`
        if (s.bw() >= (BITS - 1))
            || ((1usize << s.bw()) != self.bw())
            || ((self.bw() << 1) != rhs.bw())
        {
            return None
        }
        let s = s.to_usize();
        let digits = digits_u(s);
        let bits = extra_u(s);
        if bits == 0 {
            // Safety: there are two nonoverlapping `Bits`, and no out-of-bounds can occur
            // because of strict checks
            unsafe {
                ptr::copy_nonoverlapping(rhs.as_ptr().add(digits), self.as_mut_ptr(), self.len());
            }
        } else if self.bw() < BITS {
            *self.first_mut() = rhs.first() >> bits;
        } else {
            // Safety: When `self.bw() >= BITS`, `digits + i + 1` can be at most
            // `self.len() + digits` which cannot reach `rhs.len()`.
            unsafe {
                const_for!(i in {0..self.len()} {
                    *self.get_unchecked_mut(i) = (rhs.get_unchecked(digits + i) >> bits)
                        | (rhs.get_unchecked(digits + i + 1) << (BITS - bits));
                });
            }
        }
        self.clear_unused_bits();
        Some(())
    }
}
