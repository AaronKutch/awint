// Note: we use `impl Into<...>` heavily instead of `U: Into<...>` generics,
// because it allows arguments to be different types

use awint_ext::{awi, awint_internals::BITS};
use Op::*;

use crate::{dag, mimick::Bits, Lineage, Op};

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
            pub fn $assign_name(&mut self, x: impl Into<dag::$prim>) {
                let x = x.into().state();
                if self.state_nzbw() == dag::$prim::get_nzbw() {
                    self.set_state(x);
                } else {
                    self.update_state(
                        self.state_nzbw(),
                        ZeroResize([x]),
                    );
                }
            }

            #[must_use]
            pub fn $to_name(&self) -> dag::$prim {
                if self.state_nzbw() == dag::$prim::get_nzbw() {
                    dag::$prim::from_state(
                        self.state(),
                    )
                } else {
                    dag::$prim::new(
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
            pub fn $assign_name(&mut self, x: impl Into<dag::$prim>) {
                let x = x.into().state();
                if self.state_nzbw() == dag::$prim::get_nzbw() {
                    self.set_state(x);
                } else {
                    self.update_state(
                        self.state_nzbw(),
                        SignResize([x]),
                    );
                }
            }

            #[must_use]
            pub fn $to_name(&self) -> dag::$prim {
                if self.state_nzbw() == dag::$prim::get_nzbw() {
                    dag::$prim::from_state(
                        self.state(),
                    )
                } else {
                    dag::$prim::new(
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
            pub fn $fn_name(&self) -> dag::bool {
                dag::bool::new($enum_var([self.state()]))
            }
        )*
    };
}

macro_rules! compare {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            #[must_use]
            pub fn $fn_name(&self, rhs: &Bits) -> Option<dag::bool> {
                if self.bw() == rhs.bw() {
                    Some(dag::bool::new($enum_var([self.state(), rhs.state()])))
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
            pub fn $fn_name(&self, rhs: &Bits) -> Option<dag::bool> {
                if self.bw() == rhs.bw() {
                    Some(dag::bool::new($enum_var([rhs.state(), self.state()])))
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
            pub fn $fn_name(&mut self, s: impl Into<dag::usize>) -> Option<()> {
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
            pub fn $fn_name(&self) -> dag::usize {
                dag::usize::new($enum_var([self.state()]))
            }
        )*
    };
}

/// # Note
///
/// These functions are all mimicks of functions for [awint_ext::Bits], except
/// for the special `opaque_` that can never be evaluated.
impl Bits {
    unary!(
        not_ Not,
        rev_ Rev,
        abs_ Abs,
    );

    binary!(
        or_ Or,
        and_ And,
        xor_ Xor,
        add_ Add,
        sub_ Sub,
        rsb_ Rsb,
    );

    zero_cast!(
        bool bool_  to_bool,
        usize usize_ to_usize,
        u8 u8_ to_u8,
        u16 u16_ to_u16,
        u32 u32_ to_u32,
        u64 u64_ to_u64,
        u128 u128_ to_u128,
    );

    sign_cast!(
        isize isize_ to_isize,
        i8 i8_ to_i8,
        i16 i16_ to_i16,
        i32 i32_ to_i32,
        i64 i64_ to_i64,
        i128 i128_ to_i128,
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
        shl_ Shl,
        lshr_ Lshr,
        ashr_ Ashr,
        rotl_ Rotl,
        rotr_ Rotr,
    );

    ref_self_output_usize!(
        lz Lz,
        tz Tz,
        sig Sig,
        count_ones CountOnes,
    );

    pub fn opaque_(&mut self) {
        self.update_state(self.state_nzbw(), Opaque(vec![self.state()]));
    }

    pub fn opaque_with_(&mut self, with: &[&Bits]) {
        let mut v = vec![self.state()];
        for x in with {
            v.push(x.state());
        }
        self.update_state(self.state_nzbw(), Opaque(v));
    }

    pub fn zero_(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awi::ExtAwi::zero(self.nzbw())),
        );
    }

    pub fn umax_(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awi::ExtAwi::umax(self.nzbw())),
        );
    }

    pub fn imax_(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awi::ExtAwi::imax(self.nzbw())),
        );
    }

    pub fn imin_(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awi::ExtAwi::imin(self.nzbw())),
        );
    }

    pub fn uone_(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awi::ExtAwi::uone(self.nzbw())),
        );
    }

    pub fn mux_(&mut self, rhs: &Self, b: impl Into<dag::bool>) -> Option<()> {
        if self.bw() == rhs.bw() {
            self.update_state(
                self.state_nzbw(),
                Mux([self.state(), rhs.state(), b.into().state()]),
            );
            Some(())
        } else {
            None
        }
    }

    #[must_use]
    pub fn lut_(&mut self, lut: &Self, inx: &Self) -> Option<()> {
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
    pub fn get(&self, inx: impl Into<dag::usize>) -> Option<dag::bool> {
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
            Some(dag::bool::new(StaticGet([self.state()], inx)))
        } else {
            Some(dag::bool::new(Get([self.state(), inx])))
        }
    }

    #[must_use]
    pub fn set(&mut self, inx: impl Into<dag::usize>, bit: impl Into<dag::bool>) -> Option<()> {
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
        to: impl Into<dag::usize>,
        rhs: &Self,
        from: impl Into<dag::usize>,
        width: impl Into<dag::usize>,
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
        to: impl Into<dag::usize>,
        rhs: &Self,
        width: impl Into<dag::usize>,
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
        from: impl Into<dag::usize>,
        width: impl Into<dag::usize>,
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
    pub fn field_width(&mut self, rhs: &Self, width: impl Into<dag::usize>) -> Option<()> {
        self.update_state(
            self.state_nzbw(),
            FieldWidth([self.state(), rhs.state(), width.into().state()]),
        );
        Some(())
    }

    #[must_use]
    pub fn field_bit(
        &mut self,
        to: impl Into<dag::usize>,
        rhs: &Self,
        from: impl Into<dag::usize>,
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

    pub fn resize_(&mut self, rhs: &Self, extension: impl Into<dag::bool>) {
        self.update_state(
            self.state_nzbw(),
            Resize([rhs.state(), extension.into().state()]),
        );
    }

    pub fn zero_resize_(&mut self, rhs: &Self) -> dag::bool {
        let b = dag::bool::new(ZeroResizeOverflow([rhs.state()], self.nzbw()));
        self.update_state(self.state_nzbw(), ZeroResize([rhs.state()]));
        b
    }

    pub fn sign_resize_(&mut self, rhs: &Self) -> dag::bool {
        let b = dag::bool::new(SignResizeOverflow([rhs.state()], self.nzbw()));
        self.update_state(self.state_nzbw(), SignResize([rhs.state()]));
        b
    }

    #[must_use]
    pub fn funnel_(&mut self, rhs: &Self, s: &Self) -> Option<()> {
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
        if (quo.bw() == rem.bw()) && (duo.bw() == div.bw()) && (quo.bw() == duo.bw()) {
            quo.update_state(quo.state_nzbw(), UQuo([duo.state(), div.state()]));
            rem.update_state(rem.state_nzbw(), URem([duo.state(), div.state()]));
            Some(())
        } else {
            None
        }
    }

    #[must_use]
    pub fn idivide(quo: &mut Self, rem: &mut Self, duo: &mut Self, div: &mut Self) -> Option<()> {
        if (quo.bw() == rem.bw()) && (duo.bw() == div.bw()) && (quo.bw() == duo.bw()) {
            quo.update_state(quo.state_nzbw(), IQuo([duo.state(), div.state()]));
            rem.update_state(rem.state_nzbw(), IRem([duo.state(), div.state()]));
            Some(())
        } else {
            None
        }
    }

    #[must_use]
    pub fn mul_add_(&mut self, lhs: &Self, rhs: &Self) -> Option<()> {
        if (self.bw() == lhs.bw()) && (lhs.bw() == rhs.bw()) {
            self.update_state(
                self.state_nzbw(),
                MulAdd([self.state(), lhs.state(), rhs.state()]),
            );
            Some(())
        } else {
            None
        }
    }

    pub fn arb_umul_add_(&mut self, lhs: &Bits, rhs: &Bits) {
        self.update_state(
            self.state_nzbw(),
            MulAdd([self.state(), lhs.state(), rhs.state()]),
        );
    }

    pub fn arb_imul_add_(&mut self, lhs: &mut Bits, rhs: &mut Bits) {
        let mut lhs = dag::ExtAwi::from_bits(lhs);
        let mut rhs = dag::ExtAwi::from_bits(rhs);
        let lhs_msb = lhs.msb();
        let rhs_msb = rhs.msb();
        lhs.neg_(lhs_msb);
        rhs.neg_(rhs_msb);
        self.neg_(lhs_msb);
        self.neg_(rhs_msb);
        self.update_state(
            self.state_nzbw(),
            MulAdd([self.state(), lhs.state(), rhs.state()]),
        );
        self.neg_(lhs_msb);
        self.neg_(rhs_msb);
    }

    pub fn inc_(&mut self, cin: impl Into<dag::bool>) -> dag::bool {
        let b = cin.into();
        let out = dag::bool::new(IncCout([self.state(), b.state()]));
        self.update_state(self.state_nzbw(), Inc([self.state(), b.state()]));
        out
    }

    pub fn dec_(&mut self, cin: impl Into<dag::bool>) -> dag::bool {
        let b = cin.into();
        let out = dag::bool::new(DecCout([self.state(), b.state()]));
        self.update_state(self.state_nzbw(), Dec([self.state(), b.state()]));
        out
    }

    pub fn neg_(&mut self, neg: impl Into<dag::bool>) {
        let b = neg.into();
        self.update_state(self.state_nzbw(), Neg([self.state(), b.state()]));
    }

    #[must_use]
    pub fn cin_sum_(
        &mut self,
        cin: impl Into<dag::bool>,
        lhs: &Self,
        rhs: &Self,
    ) -> Option<(dag::bool, dag::bool)> {
        if (self.bw() == lhs.bw()) && (lhs.bw() == rhs.bw()) {
            let b = cin.into();
            let out = Some((
                dag::bool::new(UnsignedOverflow([b.state(), lhs.state(), rhs.state()])),
                dag::bool::new(SignedOverflow([b.state(), lhs.state(), rhs.state()])),
            ));
            self.update_state(
                self.state_nzbw(),
                CinSum([b.state(), lhs.state(), rhs.state()]),
            );
            out
        } else {
            None
        }
    }
}

#[doc(hidden)]
impl Bits {
    #[must_use]
    pub const fn must_use<T>(t: T) -> T {
        t
    }

    pub const fn unstable_raw_digits(w: usize) -> usize {
        awint_ext::awint_internals::raw_digits(w)
    }

    // TODO for now assume they pass

    pub fn unstable_le_checks<const N: usize>(
        _le_checks: [(impl Into<dag::usize>, impl Into<dag::usize>); N],
    ) -> Option<()> {
        Some(())
    }

    pub fn unstable_common_checks<const N: usize, const M: usize>(
        _common_cw: impl Into<dag::usize>,
        _ge: [impl Into<dag::usize>; N],
        _eq: [impl Into<dag::usize>; M],
    ) -> Option<()> {
        Some(())
    }

    pub fn unstable_max<const N: usize>(x: [awi::usize; N]) -> awi::usize {
        let mut max = x[0];
        for i in 1..N {
            if x[i] > max {
                max = x[i];
            }
        }
        max
    }
}
