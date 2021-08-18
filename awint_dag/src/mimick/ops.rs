// Note: we use `impl Into<...>` heavily instead of `U: Into<...>` generics,
// because it allows arguments to be different types

use awint_internals::BITS;
use Op::*;

use crate::{
    mimick::{Bits, ConstBwLineage, Lineage, State},
    primitive as prim, Op,
};

macro_rules! unary {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&mut self) {
                self.state = State::new(self.nzbw(), $enum_var, vec![self.state()]);
            }
        )*
    };
}

macro_rules! binary {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&mut self, rhs: &Self) -> Option<()> {
                if self.bw() == rhs.bw() {
                    self.state = State::new(
                        self.nzbw(),
                        $enum_var,
                        vec![self.state(), rhs.state()]
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
                self.state = State::new(
                    self.nzbw(),
                    ZeroResize(self.nzbw()),
                    vec![x.into().state()]
                );
            }

            pub fn $to_name(&self) -> prim::$prim {
                prim::$prim::new(ZeroResize(prim::$prim::hidden_const_nzbw()), vec![self.state()])
            }
        )*
    };
}

macro_rules! sign_cast {
    ($($prim:ident $assign_name:ident $to_name:ident),*,) => {
        $(
            pub fn $assign_name(&mut self, x: impl Into<prim::$prim>) {
                self.state = State::new(
                    self.nzbw(),
                    SignResize(self.nzbw()),
                    vec![x.into().state()]
                );
            }

            pub fn $to_name(&self) -> prim::$prim {
                prim::$prim::new(SignResize(prim::$prim::hidden_const_nzbw()), vec![self.state()])
            }
        )*
    };
}

macro_rules! ref_self_output_bool {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&self) -> prim::bool {
                prim::bool::new($enum_var, vec![self.state()])
            }
        )*
    };
}

macro_rules! compare {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&self, rhs: &Bits) -> Option<prim::bool> {
                if self.bw() == rhs.bw() {
                    Some(prim::bool::new($enum_var, vec![self.state(), rhs.state()]))
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
            pub fn $fn_name(&self, rhs: &Bits) -> Option<prim::bool> {
                if self.bw() == rhs.bw() {
                    Some(prim::bool::new($enum_var, vec![rhs.state(), self.state()]))
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
            pub fn $fn_name(&mut self, s: impl Into<prim::usize>) -> Option<()> {
                self.state = State::new(
                    self.nzbw(),
                    $enum_var,
                    vec![self.state(), s.into().state()]
                );
                Some(())
            }
        )*
    };
}

macro_rules! ref_self_output_usize {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&self) -> prim::usize {
                prim::usize::new($enum_var, vec![self.state()])
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
        neg_assign Neg,
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
        count_ones CountOnes,
    );

    pub fn opaque_assign(&mut self) {
        self.state = State::new(self.nzbw(), Opaque, vec![]);
    }

    pub fn zero_assign(&mut self) {
        self.state = State::new(
            self.nzbw(),
            Op::Literal(awint_ext::ExtAwi::zero(self.nzbw())),
            vec![],
        );
    }

    pub fn umax_assign(&mut self) {
        self.state = State::new(
            self.nzbw(),
            Op::Literal(awint_ext::ExtAwi::umax(self.nzbw())),
            vec![],
        );
    }

    pub fn imax_assign(&mut self) {
        self.state = State::new(
            self.nzbw(),
            Op::Literal(awint_ext::ExtAwi::imax(self.nzbw())),
            vec![],
        );
    }

    pub fn imin_assign(&mut self) {
        self.state = State::new(
            self.nzbw(),
            Op::Literal(awint_ext::ExtAwi::imin(self.nzbw())),
            vec![],
        );
    }

    pub fn uone_assign(&mut self) {
        self.state = State::new(
            self.nzbw(),
            Op::Literal(awint_ext::ExtAwi::uone(self.nzbw())),
            vec![],
        );
    }

    pub fn copy_assign(&mut self, rhs: &Self) -> Option<()> {
        if self.bw() == rhs.bw() {
            self.state = State::new(self.nzbw(), Copy, vec![rhs.state()]);
            Some(())
        } else {
            None
        }
    }

    pub fn lut(&mut self, lut: &Self, inx: &Self) -> Option<()> {
        if inx.bw() < BITS {
            if let Some(lut_len) = (1usize << inx.bw()).checked_mul(self.bw()) {
                if lut_len == lut.bw() {
                    self.state = State::new(self.nzbw(), Lut(self.nzbw()), vec![
                        lut.state(),
                        inx.state(),
                    ]);
                    return Some(())
                }
            }
        }
        None
    }

    pub fn lut_set(&mut self, entry: &Self, inx: &Self) -> Option<()> {
        if inx.bw() < BITS {
            if let Some(lut_len) = (1usize << inx.bw()).checked_mul(entry.bw()) {
                if lut_len == self.bw() {
                    self.state = State::new(self.nzbw(), LutSet, vec![
                        self.state(),
                        entry.state(),
                        inx.state(),
                    ]);
                    return Some(())
                }
            }
        }
        None
    }

    pub fn field(
        &mut self,
        to: impl Into<prim::usize>,
        rhs: &Self,
        from: impl Into<prim::usize>,
        width: impl Into<prim::usize>,
    ) -> Option<()> {
        self.state = State::new(self.nzbw(), Field, vec![
            self.state(),
            to.into().state(),
            rhs.state(),
            from.into().state(),
            width.into().state(),
        ]);
        Some(())
    }

    pub fn resize_assign(&mut self, rhs: &Self, extension: impl Into<prim::bool>) {
        self.state = State::new(self.nzbw(), Resize(self.nzbw()), vec![
            rhs.state(),
            extension.into().state(),
        ]);
    }

    pub fn zero_resize_assign(&mut self, rhs: &Self) -> prim::bool {
        let b = prim::bool::new(ZeroResizeOverflow(self.nzbw()), vec![rhs.state()]);
        self.state = State::new(self.nzbw(), ZeroResize(self.nzbw()), vec![rhs.state()]);
        b
    }

    pub fn sign_resize_assign(&mut self, rhs: &Self) -> prim::bool {
        let b = prim::bool::new(SignResizeOverflow(self.nzbw()), vec![rhs.state()]);
        self.state = State::new(self.nzbw(), SignResize(self.nzbw()), vec![rhs.state()]);
        b
    }

    pub fn funnel(&mut self, rhs: &Self, s: &Self) -> Option<()> {
        if (s.bw() >= (BITS - 1))
            || ((1usize << s.bw()) != self.bw())
            || ((self.bw() << 1) != rhs.bw())
        {
            None
        } else {
            self.state = State::new(self.nzbw(), Funnel, vec![rhs.state(), s.state()]);
            Some(())
        }
    }

    pub fn udivide(quo: &mut Self, rem: &mut Self, duo: &Self, div: &Self) -> Option<()> {
        quo.state = State::new(quo.nzbw(), UQuo, vec![duo.state(), div.state()]);
        rem.state = State::new(rem.nzbw(), URem, vec![duo.state(), div.state()]);
        Some(())
    }

    pub fn idivide(quo: &mut Self, rem: &mut Self, duo: &Self, div: &Self) -> Option<()> {
        quo.state = State::new(quo.nzbw(), IQuo, vec![duo.state(), div.state()]);
        rem.state = State::new(rem.nzbw(), IRem, vec![duo.state(), div.state()]);
        Some(())
    }

    pub fn mul_add_triop(&mut self, lhs: &Self, rhs: &Self) -> Option<()> {
        self.state = State::new(self.nzbw(), MulAdd, vec![lhs.state(), rhs.state()]);
        Some(())
    }

    pub fn inc_assign(&mut self, cin: impl Into<prim::bool>) -> prim::bool {
        let b = cin.into();
        let out = prim::bool::new(IncCout, vec![self.state(), b.state()]);
        self.state = State::new(self.nzbw(), Inc, vec![self.state(), b.state()]);
        out
    }

    pub fn dec_assign(&mut self, cin: impl Into<prim::bool>) -> prim::bool {
        let b = cin.into();
        let out = prim::bool::new(DecCout, vec![self.state(), b.state()]);
        self.state = State::new(self.nzbw(), Dec, vec![self.state(), b.state()]);
        out
    }

    pub fn cin_sum_triop(
        &mut self,
        cin: impl Into<prim::bool>,
        lhs: &Self,
        rhs: &Self,
    ) -> Option<(prim::bool, prim::bool)> {
        let b = cin.into();
        let out = Some((
            prim::bool::new(UnsignedOverflow, vec![b.state(), lhs.state(), rhs.state()]),
            prim::bool::new(SignedOverflow, vec![b.state(), lhs.state(), rhs.state()]),
        ));
        self.state = State::new(self.nzbw(), CinSum, vec![
            b.state(),
            lhs.state(),
            rhs.state(),
        ]);
        out
    }

    #[doc(hidden)]
    pub fn unstable_lt_checks<const N: usize>(
        _lt_checks: [(impl Into<prim::usize>, impl Into<prim::usize>); N],
    ) -> Option<()> {
        Some(())
    }

    #[doc(hidden)]
    pub fn unstable_common_lt_checks<const N: usize>(
        _common_lhs: impl Into<prim::usize>,
        _rhss: [impl Into<prim::usize>; N],
    ) -> Option<()> {
        Some(())
    }

    #[doc(hidden)]
    pub fn unstable_common_ne_checks<const N: usize>(
        _common_lhs: impl Into<prim::usize>,
        _rhss: [impl Into<prim::usize>; N],
    ) -> Option<()> {
        Some(())
    }
}
