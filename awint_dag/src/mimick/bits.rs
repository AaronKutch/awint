use std::{num::NonZeroUsize, rc::Rc};

use awint_internals::BITS;
use Op::*;

use crate::mimick::{primitive as prim, Lineage, Op};

/// Mimicking `awint_core::Bits`
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Bits {
    bw: NonZeroUsize,
    op: Rc<Op>,
}

impl Bits {
    /// Initializes a new `Bits` with bitwidth `bw` and initial `Op` `init_op`.
    pub(crate) fn new(bw: NonZeroUsize, op: Op) -> Self {
        Self {
            bw,
            op: Rc::new(op),
        }
    }

    pub fn nzbw(&self) -> NonZeroUsize {
        self.bw
    }

    pub fn bw(&self) -> usize {
        self.bw.get()
    }

    pub fn const_as_ref(&self) -> &Self {
        self
    }

    pub fn const_as_mut(&mut self) -> &mut Self {
        self
    }
}

impl Lineage for Bits {
    fn nzbw(&self) -> NonZeroUsize {
        self.bw
    }

    fn op(&self) -> Rc<Op> {
        Rc::clone(&self.op)
    }

    fn op_mut(&mut self) -> &mut Rc<Op> {
        &mut self.op
    }
}

macro_rules! unary_bw {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&mut self) {
                self.update($enum_var(self.nzbw()));
            }
        )*
    };
}

macro_rules! unary {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&mut self) {
                self.update($enum_var(self.op()));
            }
        )*
    };
}

/// note: if `self.bw() != rhs.bw()`, the function is assumed to not mutate
/// `self`
macro_rules! binary {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&mut self, rhs: &Self) -> Option<()> {
                if self.bw() == rhs.bw() {
                    self.update($enum_var(self.op(), rhs.op()));
                    Some(())
                } else {
                    // unequal bitwidths results in no changes
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
                self.update(ZeroResizeAssign(self.nzbw(), x.into().op()))
            }

            pub fn $to_name(&self) -> prim::$prim {
                prim::$prim::from_op(ZeroResizeAssign(self.nzbw(), self.op()))
            }
        )*
    };
}

macro_rules! sign_cast {
    ($($prim:ident $assign_name:ident $to_name:ident),*,) => {
        $(
            pub fn $assign_name<I>(&mut self, x: I) where I: Into<prim::$prim> {
                self.update(SignResizeAssign(self.nzbw(), x.into().op()))
            }

            pub fn $to_name(&self) -> prim::$prim {
                prim::$prim::from_op(SignResizeAssign(self.nzbw(), self.op()))
            }
        )*
    };
}

macro_rules! ref_self_output_bool {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&self) -> prim::bool {
                prim::bool::from_op($enum_var(self.op()))
            }
        )*
    };
}

macro_rules! compare {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&self, rhs: &Bits) -> Option<prim::bool> {
                if self.bw() == rhs.bw() {
                    Some(prim::bool::from_op($enum_var(self.op(), rhs.op())))
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
                self.update($enum_var(self.op(), s.into().op()));
                Some(())
            }
        )*
    };
}

macro_rules! ref_self_output_usize {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&self) -> prim::usize {
                prim::usize::from_op($enum_var(self.op()))
            }
        )*
    };
}

/// # Note
///
/// These functions are all mirrors of functions for [awint_core::Bits] (see
/// this link for more documentation).
impl Bits {
    unary_bw!(
        opaque_assign OpaqueAssign, // unique to `mimick::Bits`
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
            self.update(CopyAssign(rhs.op()));
            Some(())
        } else {
            None
        }
    }

    pub fn lut(&mut self, lut: &Self, inx: &Self) -> Option<()> {
        if inx.bw() < BITS {
            if let Some(lut_len) = (1usize << inx.bw()).checked_mul(self.bw()) {
                if lut_len == lut.bw() {
                    self.update(Lut(lut.op(), inx.op()));
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
        // TODO what to do about not being able to compare
        self.update(Field(
            self.op(),
            to.into().op(),
            rhs.op(),
            from.into().op(),
            width.into().op(),
        ));
        Some(())
    }

    pub fn resize_assign<B>(&mut self, rhs: &Self, extension: B)
    where
        B: Into<prim::bool>,
    {
        self.update(ResizeAssign(self.nzbw(), rhs.op(), extension.into().op()));
    }

    pub fn funnel(&mut self, rhs: &Self, s: &Self) -> Option<()> {
        if (s.bw() >= (BITS - 1))
            || ((1usize << s.bw()) != self.bw())
            || ((self.bw() << 1) != rhs.bw())
        {
            None
        } else {
            self.update(Funnel(rhs.op(), s.op()));
            Some(())
        }
    }

    pub fn udivide(quo: &mut Self, rem: &mut Self, duo: &Self, div: &Self) -> Option<()> {
        quo.update(UQuoAssign(duo.op(), div.op()));
        rem.update(URemAssign(duo.op(), div.op()));
        Some(())
    }

    pub fn idivide(quo: &mut Self, rem: &mut Self, duo: &Self, div: &Self) -> Option<()> {
        quo.update(IQuoAssign(duo.op(), div.op()));
        rem.update(IRemAssign(duo.op(), div.op()));
        Some(())
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
