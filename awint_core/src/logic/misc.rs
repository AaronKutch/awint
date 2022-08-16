use core::ptr;

use awint_internals::*;
use const_fn::const_fn;

use crate::Bits;

/// # Miscellanious
impl Bits {
    /// Returns the least significant bit
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn lsb(&self) -> bool {
        (self.first() & 1) != 0
    }

    /// Returns the most significant bit
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn msb(&self) -> bool {
        if self.extra() == 0 {
            (self.last() as isize) < 0
        } else {
            (self.last() & (1 << (self.extra() - 1))) != 0
        }
    }

    /// Gets the bit at `inx` bits from the least significant bit, returning
    /// `None` if `inx >= self.bw()`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn get(&self, inx: usize) -> Option<bool> {
        if inx >= self.bw() {
            None
        } else {
            unsafe { Some((self.get_unchecked(digits_u(inx)) & (1 << extra_u(inx))) != 0) }
        }
    }

    /// Sets the bit at `inx` bits from the least significant bit, returning
    /// `None` if `inx >= self.bw()`
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn set(&mut self, inx: usize, bit: bool) -> Option<()> {
        if inx >= self.bw() {
            None
        } else {
            unsafe {
                let x = self.get_unchecked(digits_u(inx));
                *self.get_unchecked_mut(digits_u(inx)) = if bit {
                    x | (1 << extra_u(inx))
                } else {
                    x & (!(1 << extra_u(inx)))
                };
            }
            Some(())
        }
    }

    /// Returns the number of leading zero bits
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn lz(&self) -> usize {
        // If unused bits are set, then the caller is going to get unexpected behavior
        // somewhere, also prevent overflow
        self.assert_cleared_unused_bits();
        const_for!(i in {0..self.len()}.rev() {
            let x = unsafe{self.get_unchecked(i)};
            if x != 0 {
                return ((self.len() - 1 - i) * BITS) + (x.leading_zeros() as usize) - self.unused();
            }
        });
        (self.len() * BITS) - self.unused()
    }

    /// Returns the number of trailing zero bits
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn tz(&self) -> usize {
        // If unused bits are set, then the caller is going to get unexpected behavior
        // somewhere, also prevent overflow
        self.assert_cleared_unused_bits();
        const_for!(i in {0..self.len()} {
            let x = unsafe{self.get_unchecked(i)};
            if x != 0 {
                return (i * BITS) + (x.trailing_zeros() as usize);
            }
        });
        (self.len() * BITS) - self.unused()
    }

    /// Returns the number of significant bits, `self.bw() - self.lz()`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn sig(&self) -> usize {
        self.bw() - self.lz()
    }

    /// Returns the number of set ones
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn count_ones(&self) -> usize {
        // If unused bits are set, then the caller is going to get unexpected behavior
        // somewhere, also prevent overflow
        self.assert_cleared_unused_bits();
        let mut ones = 0;
        const_for!(i in {0..self.len()} {
            let x = unsafe{self.get_unchecked(i)};
            ones += x.count_ones() as usize;
        });
        ones
    }

    /// "Fielding" bitfields with targeted copy assigns. The bitwidths of `self`
    /// and `rhs` do not have to be equal, but the inputs must collectively obey
    /// `width <= self.bw() && width <= rhs.bw() && to <= (self.bw() - width)
    /// && from <= (rhs.bw() - width)` or else `None` is
    /// returned. `width` can be zero, in which case this function just checks
    /// the input correctness and does not mutate `self`.
    ///
    /// This function works by copying a `width` sized bitfield from `rhs` at
    /// bitposition `from` and overwriting `width` bits at bitposition `to` in
    /// `self`. Only the `width` bits in `self` are mutated, any bits before and
    /// after the bitfield are left unchanged.
    ///
    /// ```
    /// use awint::{Bits, inlawi, InlAwi};
    /// // As an example, two hexadecimal digits will be overwritten
    /// // starting with the 12th digit in `y` using a bitfield with
    /// // value 0x42u8 extracted from `x`.
    /// let x = inlawi!(0x11142111u50);
    /// // the underscores are just for emphasis
    /// let mut y = inlawi!(0xfd_ec_ba9876543210u100);
    /// // from `x` digit place 3, we copy 2 digits to `y` digit place 12.
    /// y.field(12 * 4, &x, 3 * 4, 2 * 4);
    /// assert_eq!(y, inlawi!(0xfd_42_ba9876543210u100));
    /// ```
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn field(&mut self, to: usize, rhs: &Self, from: usize, width: usize) -> Option<()> {
        let bw_digits = digits_u(width);
        let bw_bits = extra_u(width);
        let from_digits = digits_u(from);
        let from_bits = extra_u(from);
        let to_digits = digits_u(to);
        let to_bits = extra_u(to);
        // we do the comparisons in this order to make sure that the subtractions do not
        // overflow
        if (width > self.bw())
            || (width > rhs.bw())
            || (to > (self.bw() - width))
            || (from > (rhs.bw() - width))
        {
            return None
        }
        if width == 0 {
            return Some(())
        }
        // since we are dealing with three different sets of digit and subdigit shifts,
        // the only sane way to do this is to make a digit aligned copy
        // (`tmp`) of the bitfield from `rhs` and then copy again to its final alignment

        // Safety: we test this vigorously in `testcrate` through `multi_bw.rs`,
        // `misc.rs`, and `macro_fuzzing.rs`. There are debug asserts for out of bounds.
        unsafe {
            if (bw_digits != 0) && (from_bits == 0) && (to_bits == 0) {
                const_for!(i in {0..bw_digits} {
                    *self.get_unchecked_mut(i + to_digits) =
                        rhs.get_unchecked(i + from_digits);
                });
                // handle last digit
                if bw_bits != 0 {
                    let to_mask = MAX << bw_bits;
                    let from_mask = !to_mask;
                    *self.get_unchecked_mut(bw_digits + to_digits) =
                        (self.get_unchecked(bw_digits + to_digits) & to_mask)
                            | (rhs.get_unchecked(bw_digits + from_digits) & from_mask);
                }
            } else if (bw_digits != 0) && (to_bits == 0) {
                const_for!(i in {0..bw_digits} {
                    *self.get_unchecked_mut(i + to_digits) =
                        rhs.get_digit(from + (i * BITS));
                });
                // handle last digit
                if bw_bits != 0 {
                    let to_mask = MAX << bw_bits;
                    let from_mask = !to_mask;
                    *self.get_unchecked_mut(bw_digits + to_digits) =
                        (self.get_unchecked(bw_digits + to_digits) & to_mask)
                            | (rhs.get_digit(from + (bw_digits * BITS)) & from_mask);
                }
            } else {
                let mut from = from;
                let mut i = to_digits;
                loop {
                    if i >= (bw_digits + to_digits) {
                        // handle the extra bits from the field
                        if bw_bits != 0 {
                            let tmp = rhs.get_digit(from);
                            // add extra masking for the extra temporary bits
                            let tmp = tmp & (MAX >> (BITS - bw_bits));
                            let mut total = to_bits + bw_bits;
                            if to_bits == 0 {
                                let mask = MAX << bw_bits;
                                *self.get_unchecked_mut(to_digits) =
                                    tmp | (self.get_unchecked(to_digits) & mask);
                            } else if total >= BITS {
                                total -= BITS;
                                let tmp = (tmp << to_bits, tmp >> (BITS - to_bits));
                                let mask = MAX >> (BITS - to_bits);
                                *self.get_unchecked_mut(i) = (self.get_unchecked(i) & mask) | tmp.0;
                                if total != 0 {
                                    // the extra bits cross a digit boundary
                                    i += 1;
                                    let mask = MAX << total;
                                    *self.get_unchecked_mut(i) =
                                        (self.get_unchecked(i) & mask) | tmp.1;
                                }
                            } else {
                                // total < BITS
                                let tmp = tmp << to_bits;
                                // Because the extra bits are fewer than BITS and they are
                                // positioned in the middle. The mask has to cover before and after
                                // the extra bits.
                                let mask = (MAX << total) | (MAX >> (BITS - to_bits));
                                *self.get_unchecked_mut(i) = (self.get_unchecked(i) & mask) | tmp;
                            }
                        }
                        break
                    }
                    let tmp = rhs.get_digit(from);
                    // shift up into new field placements
                    let tmp = (tmp << to_bits, tmp >> (BITS - to_bits));
                    // mask
                    let mask1 = MAX << to_bits;
                    // because the partial field is one `usize` long
                    let mask0 = !mask1;
                    *self.get_unchecked_mut(i) = (self.get_unchecked(i) & mask0) | tmp.0;
                    i += 1;
                    from += BITS;
                    // this incurs more stores to `self` than necessary,
                    // but the alternative is even more complex
                    *self.get_unchecked_mut(i) = (self.get_unchecked(i) & mask1) | tmp.1;
                }
            }
        }
        Some(())
    }

    /// A specialization of [Bits::field] with `from` set to 0.
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn field_to(&mut self, to: usize, rhs: &Self, width: usize) -> Option<()> {
        let bw_digits = digits_u(width);
        let bw_bits = extra_u(width);
        let to_digits = digits_u(to);
        let to_bits = extra_u(to);
        // we do the comparisons in this order to make sure that the subtractions do not
        // overflow
        if (width > self.bw()) || (width > rhs.bw()) || (to > (self.bw() - width)) {
            return None
        }
        if width == 0 {
            return Some(())
        }
        unsafe {
            if (bw_digits != 0) && (to_bits == 0) {
                const_for!(i in {0..bw_digits} {
                    *self.get_unchecked_mut(i + to_digits) =
                        rhs.get_unchecked(i);
                });
                // handle last digit
                if bw_bits != 0 {
                    let to_mask = MAX << bw_bits;
                    let from_mask = !to_mask;
                    *self.get_unchecked_mut(bw_digits + to_digits) =
                        (self.get_unchecked(bw_digits + to_digits) & to_mask)
                            | (rhs.get_unchecked(bw_digits) & from_mask);
                }
            } else {
                let mut i = to_digits;
                loop {
                    if i >= (bw_digits + to_digits) {
                        // handle the extra bits from the field
                        if bw_bits != 0 {
                            let tmp = rhs.get_unchecked(i - to_digits);
                            // add extra masking for the extra temporary bits
                            let tmp = tmp & (MAX >> (BITS - bw_bits));
                            let mut total = to_bits + bw_bits;
                            if to_bits == 0 {
                                let mask = MAX << bw_bits;
                                *self.get_unchecked_mut(to_digits) =
                                    tmp | (self.get_unchecked(to_digits) & mask);
                            } else if total >= BITS {
                                total -= BITS;
                                let tmp = (tmp << to_bits, tmp >> (BITS - to_bits));
                                let mask = MAX >> (BITS - to_bits);
                                *self.get_unchecked_mut(i) = (self.get_unchecked(i) & mask) | tmp.0;
                                if total != 0 {
                                    // the extra bits cross a digit boundary
                                    i += 1;
                                    let mask = MAX << total;
                                    *self.get_unchecked_mut(i) =
                                        (self.get_unchecked(i) & mask) | tmp.1;
                                }
                            } else {
                                // total < BITS
                                let tmp = tmp << to_bits;
                                // Because the extra bits are fewer than BITS and they are
                                // positioned in the middle. The mask has to cover before and after
                                // the extra bits.
                                let mask = (MAX << total) | (MAX >> (BITS - to_bits));
                                *self.get_unchecked_mut(i) = (self.get_unchecked(i) & mask) | tmp;
                            }
                        }
                        break
                    }
                    let tmp = rhs.get_unchecked(i - to_digits);
                    // shift up into new field placements
                    let tmp = (tmp << to_bits, tmp >> (BITS - to_bits));
                    // mask
                    let mask1 = MAX << to_bits;
                    // because the partial field is one `usize` long
                    let mask0 = !mask1;
                    *self.get_unchecked_mut(i) = (self.get_unchecked(i) & mask0) | tmp.0;
                    i += 1;
                    // this incurs more stores to `self` than necessary,
                    // but the alternative is even more complex
                    *self.get_unchecked_mut(i) = (self.get_unchecked(i) & mask1) | tmp.1;
                }
            }
        }
        Some(())
    }

    /// A specialization of [Bits::field] with `to` set to 0.
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn field_from(&mut self, rhs: &Self, from: usize, width: usize) -> Option<()> {
        let bw_digits = digits_u(width);
        let bw_bits = extra_u(width);
        let from_digits = digits_u(from);
        let from_bits = extra_u(from);
        // we do the comparisons in this order to make sure that the subtractions do not
        // overflow
        if (width > self.bw()) || (width > rhs.bw()) || (from > (rhs.bw() - width)) {
            return None
        }
        if width == 0 {
            return Some(())
        }
        unsafe {
            if (bw_digits != 0) && (from_bits == 0) {
                const_for!(i in {0..bw_digits} {
                    *self.get_unchecked_mut(i) =
                        rhs.get_unchecked(i + from_digits);
                });
                // handle last digit
                if bw_bits != 0 {
                    let to_mask = MAX << bw_bits;
                    let from_mask = !to_mask;
                    *self.get_unchecked_mut(bw_digits) = (self.get_unchecked(bw_digits) & to_mask)
                        | (rhs.get_unchecked(bw_digits + from_digits) & from_mask);
                }
            } else if bw_digits != 0 {
                const_for!(i in {0..bw_digits} {
                    *self.get_unchecked_mut(i) =
                        rhs.get_digit(from + (i * BITS));
                });
                // handle last digit
                if bw_bits != 0 {
                    let to_mask = MAX << bw_bits;
                    let from_mask = !to_mask;
                    *self.get_unchecked_mut(bw_digits) = (self.get_unchecked(bw_digits) & to_mask)
                        | (rhs.get_digit(from + (bw_digits * BITS)) & from_mask);
                }
            } else {
                let mut from = from;
                let mut i = 0;
                loop {
                    if i >= bw_digits {
                        // handle the extra bits from the field
                        if bw_bits != 0 {
                            let tmp = rhs.get_digit(from);
                            // add extra masking for the extra temporary bits
                            let tmp = tmp & (MAX >> (BITS - bw_bits));
                            let mask = MAX << bw_bits;
                            *self.first_mut() = tmp | (self.first() & mask);
                        }
                        break
                    }
                    *self.get_unchecked_mut(i) = rhs.get_digit(from);
                    i += 1;
                    from += BITS;
                }
            }
        }
        Some(())
    }

    /// A specialization of [Bits::field] with `to` and `from` set to 0.
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn field_width(&mut self, rhs: &Self, width: usize) -> Option<()> {
        if (width > self.bw()) || (width > rhs.bw()) {
            return None
        }
        let bw_digits = digits_u(width);
        let bw_bits = extra_u(width);
        unsafe {
            ptr::copy_nonoverlapping(rhs.as_ptr(), self.as_mut_ptr(), bw_digits);
            // last digit
            if bw_bits != 0 {
                let to_mask = MAX << bw_bits;
                let from_mask = !to_mask;
                *self.get_unchecked_mut(bw_digits) = (self.get_unchecked(bw_digits) & to_mask)
                    | (rhs.get_unchecked(bw_digits) & from_mask);
            }
        }
        Some(())
    }

    /// A specialization of [Bits::field] with `width` set to 1.
    #[inline]
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn field_bit(&mut self, to: usize, rhs: &Bits, from: usize) -> Option<()> {
        if let Some(b) = rhs.get(from) {
            self.set(to, b)
        } else {
            None
        }
    }

    /// Copy entry from lookup table. Copies a `self.bw()` sized bitfield from
    /// `lut` at bit position `inx.to_usize() * self.bw()`. If `lut.bw() !=
    /// (self.bw() * (2^inx.bw()))`, `None` will be returned.
    ///
    /// ```
    /// use awint::{Bits, inlawi, InlAwi};
    /// let mut out = inlawi!(0u10);
    /// // lookup table consisting of 4 10-bit entries
    /// let lut = inlawi!(4u10, 3u10, 2u10, 1u10);
    /// // the indexer has to have a bitwidth of 2 to index 2^2 = 4 entries
    /// let mut inx = inlawi!(0u2);
    ///
    /// // get the third entry (this is using zero indexing)
    /// inx.usize_assign(2);
    /// out.lut(&lut, &inx).unwrap();
    /// assert_eq!(out, inlawi!(3u10));
    /// ```
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn lut(&mut self, lut: &Self, inx: &Self) -> Option<()> {
        // because we later call `inx.to_usize()` and assume that it fits within
        // `inx.bw()`
        inx.assert_cleared_unused_bits();
        // make sure the left shift does not overflow
        if inx.bw() < BITS {
            if let Some(lut_len) = (1usize << inx.bw()).checked_mul(self.bw()) {
                if lut_len == lut.bw() {
                    let index = inx.to_usize().wrapping_mul(self.bw());
                    let digits = digits_u(index);
                    let bits = extra_u(index);
                    let self_bits = extra_u(self.bw());
                    // Safety: Because of the strict bitwidths of `self`, `lut`, and `inx`, the
                    // value of `inx` cannot index beyond the width of `lut`.
                    unsafe {
                        if bits == 0 {
                            const_for!(i in {0..self.len()} {
                                *self.get_unchecked_mut(i) = lut.get_unchecked(digits + i);
                            });
                        } else {
                            const_for!(i in {0..(self.len() - 1)} {
                                *self.get_unchecked_mut(i) = (lut.get_unchecked(digits + i) >> bits)
                                | (lut.get_unchecked(digits + i + 1) << (BITS - bits));
                            });
                            if (bits + self_bits) > BITS {
                                // this is tricky, because the extra bits from `self` and `index`
                                // can combine to push the end of
                                // the bitfield over a digit boundary
                                *self.last_mut() = (lut.get_unchecked(digits + self.len() - 1)
                                    >> bits)
                                    | (lut.get_unchecked(digits + self.len()) << (BITS - bits));
                            } else {
                                *self.last_mut() =
                                    lut.get_unchecked(digits + self.len() - 1) >> bits;
                            }
                        }
                    }
                    self.clear_unused_bits();
                    return Some(())
                }
            }
        }
        None
    }

    /// Set entry in lookup table. The inverse of [Bits::lut], this uses `entry`
    /// as a bitfield to overwrite part of `self` at bit position
    /// `inx.to_usize() * entry.bw()`. If `self.bw() != (entry.bw() *
    /// (2^inx.bw()))`, `None` will be returned.
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn lut_set(&mut self, entry: &Self, inx: &Self) -> Option<()> {
        // because we later call `inx.to_usize()` and assume that it fits within
        // `inx.bw()`
        inx.assert_cleared_unused_bits();
        // make sure the left shift does not overflow
        if inx.bw() < BITS {
            if let Some(lut_len) = (1usize << inx.bw()).checked_mul(entry.bw()) {
                if lut_len == self.bw() {
                    let index = inx.to_usize().wrapping_mul(entry.bw());
                    let inx_digits = digits_u(index);
                    let inx_bits = extra_u(index);
                    let entry_digits = digits_u(entry.bw());
                    let entry_bits = extra_u(entry.bw());
                    // Safety: Because of the strict bitwidths of `self`, `lut`, and `inx`, the
                    // value of `inx` cannot index beyond the width of `lut`. We also apply
                    // extensive testing in `testcrate`.
                    unsafe {
                        if (entry_digits != 0) && (inx_bits == 0) {
                            const_for!(i in {0..entry_digits} {
                                *self.get_unchecked_mut(i + inx_digits) =
                                    entry.get_unchecked(i);
                            });
                            // handle last digit
                            if entry_bits != 0 {
                                // `entry.get_unchecked(entry_digits)` is the same as
                                // `entry.last()`, but it can produce faster assembly in this case.
                                let to_mask = MAX << entry_bits;
                                let from_mask = !to_mask;
                                *self.get_unchecked_mut(entry_digits + inx_digits) =
                                    (self.get_unchecked(entry_digits + inx_digits) & to_mask)
                                        | (entry.get_unchecked(entry_digits) & from_mask);
                            }
                        } else {
                            let mut i = 0;
                            loop {
                                if i >= entry_digits {
                                    // handle the extra bits from the field
                                    if entry_bits != 0 {
                                        let tmp = entry.get_unchecked(entry_digits);
                                        let mut total = inx_bits + entry_bits;
                                        if inx_bits == 0 {
                                            let mask = MAX << entry_bits;
                                            *self.get_unchecked_mut(inx_digits) =
                                                tmp | (self.get_unchecked(inx_digits) & mask);
                                        } else if total >= BITS {
                                            total -= BITS;
                                            let tmp = (tmp << inx_bits, tmp >> (BITS - inx_bits));
                                            let mask = MAX >> (BITS - inx_bits);
                                            *self.get_unchecked_mut(i + inx_digits) =
                                                (self.get_unchecked(i + inx_digits) & mask) | tmp.0;
                                            if total != 0 {
                                                // the extra bits cross a digit boundary
                                                i += 1;
                                                let mask = MAX << total;
                                                *self.get_unchecked_mut(i + inx_digits) =
                                                    (self.get_unchecked(i + inx_digits) & mask)
                                                        | tmp.1;
                                            }
                                        } else {
                                            // total < BITS
                                            let tmp = tmp << inx_bits;
                                            // Because the extra bits are fewer than BITS and they
                                            // are
                                            // positioned in the middle. The mask has to cover
                                            // before and after
                                            // the extra bits.
                                            let mask = (MAX << total) | (MAX >> (BITS - inx_bits));
                                            *self.get_unchecked_mut(i + inx_digits) =
                                                (self.get_unchecked(i + inx_digits) & mask) | tmp;
                                        }
                                    }
                                    break
                                }
                                let tmp = entry.get_unchecked(i);
                                // shift up into new field placements
                                let tmp = (tmp << inx_bits, tmp >> (BITS - inx_bits));
                                // mask
                                let mask1 = MAX << inx_bits;
                                // because the partial field is one `usize` long
                                let mask0 = !mask1;
                                *self.get_unchecked_mut(i + inx_digits) =
                                    (self.get_unchecked(i + inx_digits) & mask0) | tmp.0;
                                i += 1;
                                // this incurs more stores to `self` than necessary,
                                // but the alternative is even more complex
                                *self.get_unchecked_mut(i + inx_digits) =
                                    (self.get_unchecked(i + inx_digits) & mask1) | tmp.1;
                            }
                        }
                    }
                    self.clear_unused_bits();
                    return Some(())
                }
            }
        }
        None
    }

    /// Multiplex by conditionally copy-assigning `rhs` to `self` if `b`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn mux_assign(&mut self, rhs: &Bits, b: bool) -> Option<()> {
        if self.bw() == rhs.bw() {
            if b {
                self.copy_assign(rhs).unwrap();
            }
            Some(())
        } else {
            None
        }
    }
}
