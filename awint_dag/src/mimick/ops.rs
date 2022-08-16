// Note: we use `impl Into<...>` heavily instead of `U: Into<...>` generics,
// because it allows arguments to be different types

use awint_internals::BITS;
use Op::*;

use super::ExtAwi;
use crate::{mimick::Bits, primitive as prim, Lineage, Op};

macro_rules! unary {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&mut self) {
                self.update_state(self.state_nzbw(), $enum_var([self.state()]));
            }
        )*
    };
}

macro_rules! binary {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            #[must_use]
            pub fn $fn_name(&mut self, rhs: &Self) -> Option<()> {
                if self.bw() == rhs.bw() {
                    self.update_state(
                        self.state_nzbw(),
                        $enum_var([self.state(), rhs.state()])
                    );
                    Some(())
                } else {
                    None
                }
            }
        )*
    };
}

macro_rules! zero_cast {
    ($($prim:ident $assign_name:ident $to_name:ident),*,) => {
        $(
            pub fn $assign_name(&mut self, x: impl Into<prim::$prim>) {
                let x = x.into().state();
                if self.state_nzbw() == prim::$prim::get_nzbw() {
                    self.set_state(x);
                } else {
                    self.update_state(
                        self.state_nzbw(),
                        ZeroResize([x]),
                    );
                }
            }

            #[must_use]
            pub fn $to_name(&self) -> prim::$prim {
                if self.state_nzbw() == prim::$prim::get_nzbw() {
                    prim::$prim::from_state(
                        self.state(),
                    )
                } else {
                    prim::$prim::new(
                        ZeroResize([self.state()]),
                    )
                }
            }
        )*
    };
}

macro_rules! sign_cast {
    ($($prim:ident $assign_name:ident $to_name:ident),*,) => {
        $(
            pub fn $assign_name(&mut self, x: impl Into<prim::$prim>) {
                let x = x.into().state();
                if self.state_nzbw() == prim::$prim::get_nzbw() {
                    self.set_state(x);
                } else {
                    self.update_state(
                        self.state_nzbw(),
                        SignResize([x]),
                    );
                }
            }

            #[must_use]
            pub fn $to_name(&self) -> prim::$prim {
                if self.state_nzbw() == prim::$prim::get_nzbw() {
                    prim::$prim::from_state(
                        self.state(),
                    )
                } else {
                    prim::$prim::new(
                        SignResize([self.state()]),
                    )
                }
            }
        )*
    };
}

macro_rules! ref_self_output_bool {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            #[must_use]
            pub fn $fn_name(&self) -> prim::bool {
                prim::bool::new($enum_var([self.state()]))
            }
        )*
    };
}

macro_rules! compare {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            #[must_use]
            pub fn $fn_name(&self, rhs: &Bits) -> Option<prim::bool> {
                if self.bw() == rhs.bw() {
                    Some(prim::bool::new($enum_var([self.state(), rhs.state()])))
                } else {
                    None
                }
            }
        )*
    };
}

macro_rules! compare_reversed {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            #[must_use]
            pub fn $fn_name(&self, rhs: &Bits) -> Option<prim::bool> {
                if self.bw() == rhs.bw() {
                    Some(prim::bool::new($enum_var([rhs.state(), self.state()])))
                } else {
                    None
                }
            }
        )*
    };
}

macro_rules! shift {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            #[must_use]
            pub fn $fn_name(&mut self, s: impl Into<prim::usize>) -> Option<()> {
                self.update_state(
                    self.state_nzbw(),
                    $enum_var([self.state(), s.into().state()])
                );
                Some(())
            }
        )*
    };
}

macro_rules! ref_self_output_usize {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            #[must_use]
            pub fn $fn_name(&self) -> prim::usize {
                prim::usize::new($enum_var([self.state()]))
            }
        )*
    };
}

/// # Note
///
/// These functions are all mirrors of functions for [awint_core::Bits], except
/// for the special `opaque_assign` that can never be evaluated.
impl Bits {
    unary!(
        not_assign Not,
        rev_assign Rev,
        abs_assign Abs,
    );

    binary!(
        or_assign Or,
        and_assign And,
        xor_assign Xor,
        add_assign Add,
        sub_assign Sub,
        rsb_assign Rsb,
    );

    zero_cast!(
        bool bool_assign  to_bool,
        usize usize_assign to_usize,
        u8 u8_assign to_u8,
        u16 u16_assign to_u16,
        u32 u32_assign to_u32,
        u64 u64_assign to_u64,
        u128 u128_assign to_u128,
    );

    sign_cast!(
        isize isize_assign to_isize,
        i8 i8_assign to_i8,
        i16 i16_assign to_i16,
        i32 i32_assign to_i32,
        i64 i64_assign to_i64,
        i128 i128_assign to_i128,
    );

    ref_self_output_bool!(
        is_zero IsZero,
        is_umax IsUmax,
        is_imax IsImax,
        is_imin IsImin,
        is_uone IsUone,
        lsb Lsb,
        msb Msb,
    );

    compare!(
        const_eq Eq,
        const_ne Ne,
        ult Ult,
        ule Ule,
        ilt Ilt,
        ile Ile,
    );

    compare_reversed!(
        ugt Ult,
        uge Ule,
        igt Ilt,
        ige Ile,
    );

    shift!(
        shl_assign Shl,
        lshr_assign Lshr,
        ashr_assign Ashr,
        rotl_assign Rotl,
        rotr_assign Rotr,
    );

    ref_self_output_usize!(
        lz Lz,
        tz Tz,
        sig Sig,
        count_ones CountOnes,
    );

    pub fn opaque_assign(&mut self) {
        self.update_state(self.state_nzbw(), Opaque(vec![self.state()]));
    }

    pub fn zero_assign(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awint_ext::ExtAwi::zero(self.nzbw())),
        );
    }

    pub fn umax_assign(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awint_ext::ExtAwi::umax(self.nzbw())),
        );
    }

    pub fn imax_assign(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awint_ext::ExtAwi::imax(self.nzbw())),
        );
    }

    pub fn imin_assign(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awint_ext::ExtAwi::imin(self.nzbw())),
        );
    }

    pub fn uone_assign(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awint_ext::ExtAwi::uone(self.nzbw())),
        );
    }

    pub fn mux_assign(&mut self, rhs: &Self, b: impl Into<prim::bool>) -> Option<()> {
        self.update_state(
            self.state_nzbw(),
            Mux([self.state(), rhs.state(), b.into().state()]),
        );
        Some(())
    }

    #[must_use]
    pub fn lut_assign(&mut self, lut: &Self, inx: &Self) -> Option<()> {
        if inx.bw() < BITS {
            if let Some(lut_len) = (1usize << inx.bw()).checked_mul(self.bw()) {
                if lut_len == lut.bw() {
                    self.update_state(self.state_nzbw(), Lut([lut.state(), inx.state()]));
                    return Some(())
                }
            }
        }
        None
    }

    #[must_use]
    pub fn lut_set(&mut self, entry: &Self, inx: &Self) -> Option<()> {
        if inx.bw() < BITS {
            if let Some(lut_len) = (1usize << inx.bw()).checked_mul(entry.bw()) {
                if lut_len == self.bw() {
                    self.update_state(
                        self.state_nzbw(),
                        LutSet([self.state(), entry.state(), inx.state()]),
                    );
                    return Some(())
                }
            }
        }
        None
    }

    #[must_use]
    pub fn get(&self, inx: impl Into<prim::usize>) -> Option<prim::bool> {
        let inx = inx.into().state();
        if let Literal(ref lit) = inx.get_state().unwrap().op {
            // optimization for the meta lowering
            let inx = lit.to_usize();
            if inx >= self.bw() {
                panic!(
                    "mimicking Bits::get({}) is out of bounds with bitwidth {}",
                    inx,
                    self.bw()
                );
            }
            Some(prim::bool::new(StaticGet([self.state()], inx)))
        } else {
            Some(prim::bool::new(Get([self.state(), inx])))
        }
    }

    #[must_use]
    pub fn set(&mut self, inx: impl Into<prim::usize>, bit: impl Into<prim::bool>) -> Option<()> {
        let inx = inx.into().state();
        if let Literal(ref lit) = inx.get_state().unwrap().op {
            // optimization for the meta lowering
            let inx = lit.to_usize();
            if inx >= self.bw() {
                panic!(
                    "mimicking Bits::set({}) is out of bounds with bitwidth {}",
                    inx,
                    self.bw()
                );
            }
            self.update_state(
                self.state_nzbw(),
                StaticSet([self.state(), bit.into().state()], inx),
            );
        } else {
            self.update_state(
                self.state_nzbw(),
                Set([self.state(), inx, bit.into().state()]),
            );
        }
        Some(())
    }

    #[must_use]
    pub fn field(
        &mut self,
        to: impl Into<prim::usize>,
        rhs: &Self,
        from: impl Into<prim::usize>,
        width: impl Into<prim::usize>,
    ) -> Option<()> {
        self.update_state(
            self.state_nzbw(),
            Field([
                self.state(),
                to.into().state(),
                rhs.state(),
                from.into().state(),
                width.into().state(),
            ]),
        );
        Some(())
    }

    #[must_use]
    pub fn field_to(
        &mut self,
        to: impl Into<prim::usize>,
        rhs: &Self,
        width: impl Into<prim::usize>,
    ) -> Option<()> {
        self.update_state(
            self.state_nzbw(),
            FieldTo([
                self.state(),
                to.into().state(),
                rhs.state(),
                width.into().state(),
            ]),
        );
        Some(())
    }

    #[must_use]
    pub fn field_from(
        &mut self,
        rhs: &Self,
        from: impl Into<prim::usize>,
        width: impl Into<prim::usize>,
    ) -> Option<()> {
        self.update_state(
            self.state_nzbw(),
            FieldFrom([
                self.state(),
                rhs.state(),
                from.into().state(),
                width.into().state(),
            ]),
        );
        Some(())
    }

    #[must_use]
    pub fn field_width(&mut self, rhs: &Self, width: impl Into<prim::usize>) -> Option<()> {
        self.update_state(
            self.state_nzbw(),
            FieldWidth([self.state(), rhs.state(), width.into().state()]),
        );
        Some(())
    }

    #[must_use]
    pub fn field_bit(
        &mut self,
        to: impl Into<prim::usize>,
        rhs: &Self,
        from: impl Into<prim::usize>,
    ) -> Option<()> {
        self.update_state(
            self.state_nzbw(),
            FieldBit([
                self.state(),
                to.into().state(),
                rhs.state(),
                from.into().state(),
            ]),
        );
        Some(())
    }

    pub fn resize_assign(&mut self, rhs: &Self, extension: impl Into<prim::bool>) {
        self.update_state(
            self.state_nzbw(),
            Resize([rhs.state(), extension.into().state()]),
        );
    }

    pub fn zero_resize_assign(&mut self, rhs: &Self) -> prim::bool {
        let b = prim::bool::new(ZeroResizeOverflow([rhs.state()], self.nzbw()));
        self.update_state(self.state_nzbw(), ZeroResize([rhs.state()]));
        b
    }

    pub fn sign_resize_assign(&mut self, rhs: &Self) -> prim::bool {
        let b = prim::bool::new(SignResizeOverflow([rhs.state()], self.nzbw()));
        self.update_state(self.state_nzbw(), SignResize([rhs.state()]));
        b
    }

    #[must_use]
    pub fn funnel(&mut self, rhs: &Self, s: &Self) -> Option<()> {
        if (s.bw() >= (BITS - 1))
            || ((1usize << s.bw()) != self.bw())
            || ((self.bw() << 1) != rhs.bw())
        {
            None
        } else {
            self.update_state(self.state_nzbw(), Funnel([rhs.state(), s.state()]));
            Some(())
        }
    }

    #[must_use]
    pub fn udivide(quo: &mut Self, rem: &mut Self, duo: &Self, div: &Self) -> Option<()> {
        quo.update_state(quo.state_nzbw(), UQuo([duo.state(), div.state()]));
        rem.update_state(rem.state_nzbw(), URem([duo.state(), div.state()]));
        Some(())
    }

    #[must_use]
    pub fn idivide(quo: &mut Self, rem: &mut Self, duo: &mut Self, div: &mut Self) -> Option<()> {
        quo.update_state(quo.state_nzbw(), IQuo([duo.state(), div.state()]));
        rem.update_state(rem.state_nzbw(), IRem([duo.state(), div.state()]));
        Some(())
    }

    #[must_use]
    pub fn mul_add_assign(&mut self, lhs: &Self, rhs: &Self) -> Option<()> {
        self.update_state(
            self.state_nzbw(),
            MulAdd([self.state(), lhs.state(), rhs.state()]),
        );
        Some(())
    }

    pub fn arb_umul_add_assign(&mut self, lhs: &Bits, rhs: &Bits) {
        self.update_state(
            self.state_nzbw(),
            MulAdd([self.state(), lhs.state(), rhs.state()]),
        );
    }

    pub fn arb_imul_add_assign(&mut self, lhs: &mut Bits, rhs: &mut Bits) {
        let mut lhs = ExtAwi::from_bits(lhs);
        let mut rhs = ExtAwi::from_bits(rhs);
        let lhs_msb = lhs.msb();
        let rhs_msb = rhs.msb();
        lhs.neg_assign(lhs_msb);
        rhs.neg_assign(rhs_msb);
        self.neg_assign(lhs_msb);
        self.neg_assign(rhs_msb);
        self.update_state(
            self.state_nzbw(),
            MulAdd([self.state(), lhs.state(), rhs.state()]),
        );
        self.neg_assign(lhs_msb);
        self.neg_assign(rhs_msb);
    }

    pub fn inc_assign(&mut self, cin: impl Into<prim::bool>) -> prim::bool {
        let b = cin.into();
        let out = prim::bool::new(IncCout([self.state(), b.state()]));
        self.update_state(self.state_nzbw(), Inc([self.state(), b.state()]));
        out
    }

    pub fn dec_assign(&mut self, cin: impl Into<prim::bool>) -> prim::bool {
        let b = cin.into();
        let out = prim::bool::new(DecCout([self.state(), b.state()]));
        self.update_state(self.state_nzbw(), Dec([self.state(), b.state()]));
        out
    }

    pub fn neg_assign(&mut self, neg: impl Into<prim::bool>) {
        let b = neg.into();
        self.update_state(self.state_nzbw(), Neg([self.state(), b.state()]));
    }

    #[must_use]
    pub fn cin_sum_assign(
        &mut self,
        cin: impl Into<prim::bool>,
        lhs: &Self,
        rhs: &Self,
    ) -> Option<(prim::bool, prim::bool)> {
        let b = cin.into();
        let out = Some((
            prim::bool::new(UnsignedOverflow([b.state(), lhs.state(), rhs.state()])),
            prim::bool::new(SignedOverflow([b.state(), lhs.state(), rhs.state()])),
        ));
        self.update_state(
            self.state_nzbw(),
            CinSum([b.state(), lhs.state(), rhs.state()]),
        );
        out
    }
}

#[doc(hidden)]
impl Bits {
    #[must_use]
    pub const fn must_use<T>(t: T) -> T {
        t
    }

    pub const fn unstable_raw_digits(bw: usize) -> usize {
        awint_internals::raw_digits(bw)
    }

    // TODO for now assume they pass

    pub fn unstable_le_checks<const N: usize>(
        _le_checks: [(impl Into<prim::usize>, impl Into<prim::usize>); N],
    ) -> Option<()> {
        Some(())
    }

    pub fn unstable_common_checks<const N: usize, const M: usize>(
        _common_cw: impl Into<prim::usize>,
        _ge: [impl Into<prim::usize>; N],
        _eq: [impl Into<prim::usize>; M],
    ) -> Option<()> {
        Some(())
    }
}
