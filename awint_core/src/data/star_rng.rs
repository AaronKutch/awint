use star_rng::StarRng;

use crate::{Bits, InlAwi};

// this is weird to be putting in `awint` itself, but this is such a useful
// feature that I use everywhere myself, and the internal design meshes nicely
// with `Bits`

impl Bits {
    /// Assigns random value to `self`.
    ///
    /// This has a guaranteed deterministic and efficient definition, unlike
    /// going through [Bits::rand_] with [StarRng].
    pub fn star_rng_(&mut self, rng: &mut StarRng) {
        self.star_rng_width_(rng, self.bw()).unwrap();
    }

    /// Assigns random value to `self[..width]`, zeroing the rest of the bits of
    /// `self`. Returns `None` if `width > self.bw()`.
    ///
    /// This has a guaranteed deterministic and efficient definition, unlike
    /// going through [Bits::rand_] with [StarRng].
    #[must_use]
    pub fn star_rng_width_(&mut self, rng: &mut StarRng, mut width: usize) -> Option<()> {
        if width > self.bw() {
            return None;
        }
        self.zero_();
        if width == 0 {
            return Some(());
        }
        let mut tmp = InlAwi::from_u32(0);
        let mut shl = 0;
        loop {
            if width < 32 {
                tmp.u32_(rng.internal_consume(width as u8));
                self.field_to(shl, &tmp, width).unwrap();
                break;
            }
            tmp.u32_(rng.internal_next_u32());
            self.field_to(shl, &tmp, 32).unwrap();
            width -= 32;
            shl += 32;
        }
        Some(())
    }

    /// This performs one step of a fuzzer where a random field of ones is
    /// ORed, ANDed, or XORed to `self`.
    ///
    /// In many cases there are issues that involve long lines of all set or
    /// unset bits, and the [Bits::star_rng_] function is unsuitable for this as
    /// `self.bw()` gets larger than a few bits. This function produces random
    /// length strings of ones and zeros concatenated together, which can
    /// rapidly probe a more structured space even for large `self`.
    ///
    /// ```
    /// use awint::awi::*;
    /// use star_rng::StarRng;
    ///
    /// let mut rng = &mut StarRng::new(7);
    /// let mut x = awi!(0u128);
    /// // this should be done in a loop with thousands of iterations,
    /// // here I have unrolled a few for example
    /// x.star_rng_linear_fuzz_step_(rng);
    /// assert_eq!(x, awi!(0x1_ffffffff_f0000000_u128));
    /// x.star_rng_linear_fuzz_step_(rng);
    /// assert_eq!(x, awi!(0x3ffff01_ffffffff_f0000000_u128));
    /// x.star_rng_linear_fuzz_step_(rng);
    /// assert_eq!(x, awi!(0x3fffcfe_00000001_f0000000_u128));
    /// x.star_rng_linear_fuzz_step_(rng);
    /// assert_eq!(x, awi!(0xc000301_fffffffe_0fffff00_u128));
    /// x.star_rng_linear_fuzz_step_(rng);
    /// assert_eq!(x, awi!(0xc_0c000301_fffffffe_0fffff00_u128));
    /// ```
    pub fn star_rng_linear_fuzz_step_(&mut self, rng: &mut StarRng) {
        let tmp0 = rng.index(self.bw()).unwrap();
        let tmp1 = rng.index(self.bw().wrapping_add(1)).unwrap();
        let r0 = core::cmp::min(tmp0, tmp1);
        let r1 = core::cmp::max(tmp0, tmp1);
        // note: it needs to be 2 parts XOR to 1 part OR and 1 part AND, the ordering
        // guarantees this
        if rng.next_bool() {
            self.range_xor_(r0..r1).unwrap();
        } else if rng.next_bool() {
            self.range_or_(r0..r1).unwrap();
        } else {
            self.range_and_(r0..r1).unwrap();
        }
    }
}
