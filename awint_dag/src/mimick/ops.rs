use awint_internals::BITS;
use Op::*;

use crate::{
    mimick::{Bits, ConstBwLineage, Lineage, State},
    primitive as prim, Op,
};

macro_rules! nullary {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&mut self) {
                self.state = State::new(self.nzbw(), $enum_var, vec![]);
            }
        )*
    };
}

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
            pub fn $assign_name<I>(&mut self, x: I) where I: Into<prim::$prim> {
                self.state = State::new(self.nzbw(), ZeroResizeAssign, vec![x.into().state()]);
            }

            pub fn $to_name(&self) -> prim::$prim {
                prim::$prim::new(ZeroResizeAssign, vec![self.state()])
            }
        )*
    };
}

macro_rules! sign_cast {
    ($($prim:ident $assign_name:ident $to_name:ident),*,) => {
        $(
            pub fn $assign_name<I>(&mut self, x: I) where I: Into<prim::$prim> {
                self.state = State::new(self.nzbw(), SignResizeAssign, vec![x.into().state()]);
            }

            pub fn $to_name(&self) -> prim::$prim {
                prim::$prim::new(SignResizeAssign, vec![self.state()])
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

macro_rules! shift {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name<U>(&mut self, s: U) -> Option<()>
            where
                U: Into<prim::usize>,
            {
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
    nullary!(
        opaque_assign OpaqueAssign,
        zero_assign ZeroAssign,
        umax_assign UmaxAssign,
        imax_assign ImaxAssign,
        imin_assign IminAssign,
        uone_assign UoneAssign,
    );

    unary!(
        not_assign NotAssign,
        rev_assign RevAssign,
        neg_assign NegAssign,
        abs_assign AbsAssign,
    );

    binary!(
        or_assign OrAssign,
        and_assign AndAssign,
        xor_assign XorAssign,
        add_assign AddAssign,
        sub_assign SubAssign,
        rsb_assign RsbAssign,
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
        const_eq ConstEq,
        const_ne ConstNe,
        ult Ult,
        ule Ule,
        ugt Ugt,
        uge Uge,
        ilt Ilt,
        ile Ile,
        igt Igt,
        ige Ige,
    );

    shift!(
        shl_assign ShlAssign,
        lshr_assign LshrAssign,
        ashr_assign AshrAssign,
        rotl_assign RotlAssign,
        rotr_assign RotrAssign,
    );

    ref_self_output_usize!(
        lz Lz,
        tz Tz,
        count_ones CountOnes,
    );

    pub fn copy_assign(&mut self, rhs: &Self) -> Option<()> {
        if self.bw() == rhs.bw() {
            self.state = State::new(self.nzbw(), CopyAssign, vec![rhs.state()]);
            Some(())
        } else {
            None
        }
    }

    pub fn lut(&mut self, lut: &Self, inx: &Self) -> Option<()> {
        if inx.bw() < BITS {
            if let Some(lut_len) = (1usize << inx.bw()).checked_mul(self.bw()) {
                if lut_len == lut.bw() {
                    self.state = State::new(self.nzbw(), Lut, vec![lut.state(), inx.state()]);
                    return Some(())
                }
            }
        }
        None
    }

    pub fn field<U>(&mut self, to: U, rhs: &Self, from: U, width: U) -> Option<()>
    where
        U: Into<prim::usize>,
    {
        self.state = State::new(self.nzbw(), Field, vec![
            self.state(),
            to.into().state(),
            rhs.state(),
            from.into().state(),
            width.into().state(),
        ]);
        Some(())
    }

    pub fn resize_assign<B>(&mut self, rhs: &Self, extension: B)
    where
        B: Into<prim::bool>,
    {
        self.state = State::new(self.nzbw(), ResizeAssign, vec![
            rhs.state(),
            extension.into().state(),
        ]);
    }

    pub fn zero_resize_assign<B>(&mut self, rhs: &Self) -> prim::bool {
        self.state = State::new(self.nzbw(), ZeroResizeAssign, vec![rhs.state()]);
        prim::bool::new(ZeroResizeAssignOverflow, vec![rhs.state()])
    }

    pub fn sign_resize_assign<B>(&mut self, rhs: &Self) -> prim::bool {
        self.state = State::new(self.nzbw(), SignResizeAssign, vec![rhs.state()]);
        prim::bool::new(SignResizeAssignOverflow, vec![rhs.state()])
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
        quo.state = State::new(quo.nzbw(), UQuoAssign, vec![duo.state(), div.state()]);
        rem.state = State::new(rem.nzbw(), URemAssign, vec![duo.state(), div.state()]);
        Some(())
    }

    pub fn idivide(quo: &mut Self, rem: &mut Self, duo: &Self, div: &Self) -> Option<()> {
        quo.state = State::new(quo.nzbw(), IQuoAssign, vec![duo.state(), div.state()]);
        rem.state = State::new(rem.nzbw(), IRemAssign, vec![duo.state(), div.state()]);
        Some(())
    }

    pub fn mul_add_triop(&mut self, lhs: &Self, rhs: &Self) -> Option<()> {
        self.state = State::new(self.nzbw(), MulAddTriop, vec![lhs.state(), rhs.state()]);
        Some(())
    }

    pub fn inc_assign<B>(&mut self, cin: B) -> prim::bool
    where
        B: Into<prim::bool>,
    {
        let b = cin.into();
        self.state = State::new(self.nzbw(), IncAssign, vec![self.state(), b.state()]);
        prim::bool::new(IncAssignCout, vec![self.state(), b.state()])
    }

    pub fn dec_assign<B>(&mut self, cin: B) -> prim::bool
    where
        B: Into<prim::bool>,
    {
        let b = cin.into();
        self.state = State::new(self.nzbw(), DecAssign, vec![self.state(), b.state()]);
        prim::bool::new(DecAssignCout, vec![self.state(), b.state()])
    }

    pub fn cin_sum_triop<B>(
        &mut self,
        cin: B,
        lhs: &Self,
        rhs: &Self,
    ) -> Option<(prim::bool, prim::bool)>
    where
        B: Into<prim::bool>,
    {
        let b = cin.into();
        self.state = State::new(self.nzbw(), CinSumTriop, vec![
            b.state(),
            lhs.state(),
            rhs.state(),
        ]);
        Some((
            prim::bool::new(UnsignedOverflow, vec![b.state(), lhs.state(), rhs.state()]),
            prim::bool::new(SignedOverflow, vec![b.state(), lhs.state(), rhs.state()]),
        ))
    }

    #[doc(hidden)]
    pub fn unstable_lt_checks<U, const N: usize>(_lt_checks: [(U, U); N]) -> Option<()>
    where
        U: Into<prim::usize>,
    {
        Some(())
    }

    #[doc(hidden)]
    pub fn unstable_common_lt_checks<U, const N: usize>(
        _common_lhs: U,
        _rhss: [usize; N],
    ) -> Option<()>
    where
        U: Into<prim::usize>,
    {
        Some(())
    }

    #[doc(hidden)]
    pub fn unstable_common_ne_checks<U, const N: usize>(_common_lhs: U, _rhss: [U; N]) -> Option<()>
    where
        U: Into<prim::usize>,
    {
        Some(())
    }
}
