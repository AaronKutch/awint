
/*
use awint::{inlawi, inlawi_ty, inlawi_zero, InlAwi};
use awint_dag::{Bits, Op};

type InlT = inlawi_ty!(8);
type InlT_plus1 = inlawi_ty!(9);

struct QuoRemStore {
    quo: InlT,
    rem: InlT,
    duo: InlT,
    div: InlT,
}

struct QuoRemBits<'a> {
    quo: &'a mut Bits,
    rem: &'a mut Bits,
    duo: &'a Bits,
    div: &'a Bits,
}

impl<'a> QuoRemBits<'a> {
    pub fn run(&mut self) {
        // extend by one bit to avoid edge cases that we would have to handle in the
        // SWAR opening
        let quo = inlawi_zero!(9);
        let rem = inlawi_zero!(9);
        let duo = inlawi_zero!(9);
        let div = inlawi_zero!(9);
        quo.resize_assign(self.quo, false);
        rem.resize_assign(self.rem, false);
        duo.resize_assign(self.duo, false);
        div.resize_assign(self.div, false);

        /*if div == 0 {
            $zero_div_fn()
        }
        if duo < div {
            return (0, duo)
        }*/

        // SWAR opening
        //let div_original = div;
        let shl = div.lz() - duo.lz();
        div.shl_assign(shl).unwrap();
        if duo.ult(div) {
            // fully normalize
            shl -= 1;
        }
        duo = duo.wrapping_sub(div);
        let mut quo = 1 << shl;
        /*if duo < div_original {
            return (quo, duo);
        }*/
        let mask = quo - 1;
        // central loop
        let div = div.wrapping_sub(1);
        let mut i = shl;
        loop {
            if i == 0 {
                break;
            }
            i -= 1;
            // note: do not factor the `wrapping_shl(1)` out before the msb check
            if duo.msb() {
                // Negated binary long division step.
                duo = duo.wrapping_shl(1).wrapping_add(div);
            } else {
                // Normal long division step.
                duo = duo.wrapping_shl(1).wrapping_sub(div);
            }
        }
        if duo.msb() {
            // Restore.
            duo = duo.wrapping_add(div);
        }
        // unpack
        quo = (duo & mask) | quo;
        duo = duo >> shl;
    }
}
*/

pub fn main() {}
