use alloc::rc::Rc;
use core::num::NonZeroUsize;

use awint_internals::BITS;
use Op::*;

use crate::{primitive as prim, Lineage, Op};

#[derive(Debug, Clone)]
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
}

impl Lineage for Bits {
    fn nzbw(&self) -> NonZeroUsize {
        self.bw
    }

    fn op(&self) -> Rc<Op> {
        self.op.clone()
    }

    fn op_mut(&mut self) -> &mut Rc<Op> {
        &mut self.op
    }
}

macro_rules! nullary {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&mut self) {
                self.update($enum_var);
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

macro_rules! to_and_assign {
    ($($prim:ident $assign_name:ident $enum_assign:ident $to_name:ident $enum_to:ident),*,) => {
        $(
            pub fn $assign_name<I>(&mut self, x: I) where I: Into<prim::$prim> {
                self.update($enum_assign(x.into()))
            }

            pub fn $to_name(&self) -> prim::$prim {
                prim::$prim::new($enum_to(self.op()))
            }
        )*
    };
}

macro_rules! ref_self_output_bool {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&self) -> prim::bool {
                prim::bool::new($enum_var(self.op()))
            }
        )*
    };
}

macro_rules! compare {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&self, rhs: &Bits) -> Option<prim::bool> {
                if self.bw() == rhs.bw() {
                    Some(prim::bool::new($enum_var(self.op(), rhs.op())))
                } else {
                    None
                }
            }
        )*
    };
}

macro_rules! ref_self_output_usize {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&self) -> prim::usize {
                prim::usize::new($enum_var(self.op()))
            }
        )*
    };
}

/// # Note
///
/// These functions are all mirrors of functions for [awint_core::Bits] (see
/// this link for more documentation).
impl Bits {
    nullary!(
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
        copy_assign CopyAssign,
        or_assign OrAssign,
        and_assign AndAssign,
        xor_assign XorAssign,
        add_assign AddAssign,
        sub_assign SubAssign,
        rsb_assign RsbAssign,
    );

    to_and_assign!(
        bool bool_assign BoolAssign to_bool ToBool,
        usize usize_assign UsizeAssign to_usize ToUsize,
        isize isize_assign IsizeAssign to_isize ToIsize,
        u8 u8_assign U8Assign to_u8 ToU8,
        i8 i8_assign I8Assign to_i8 ToI8,
        u16 u16_assign U16Assign to_u16 ToU16,
        i16 i16_assign I16Assign to_i16 ToI16,
        u32 u32_assign U32Assign to_u32 ToU32,
        i32 i32_assign I32Assign to_i32 ToI32,
        u64 u64_assign U64Assign to_u64 ToU64,
        i64 i64_assign I64Assign to_i64 ToI64,
        u128 u128_assign U128Assign to_u128 ToU128,
        i128 i128_assign I128Assign to_i128 ToI128,
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

    ref_self_output_usize!(
        lz Lz,
        tz Tz,
        count_ones CountOnes,
    );

    pub fn lut(&mut self, lut: &Self, inx: &Self) -> Option<()> {
        if inx.bw() < BITS {
            if let Some(lut_len) = (1usize << inx.bw()).checked_mul(self.bw()) {
                if lut_len == lut.bw() {
                    self.update(Lut(self.op(), lut.op(), inx.op()));
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
            to.into(),
            rhs.op(),
            from.into(),
            width.into(),
        ));
        Some(())
    }

    pub fn resize_assign<B>(&mut self, rhs: &Self, extension: B)
    where
        B: Into<prim::bool>,
    {
        self.update(ResizeAssign(self.op(), rhs.op(), extension.into()));
    }

    pub fn funnel(&mut self, rhs: &Self, s: &Self) -> Option<()> {
        if (s.bw() >= (BITS - 1))
            || ((1usize << s.bw()) != self.bw())
            || ((self.bw() << 1) != rhs.bw())
        {
            None
        } else {
            self.update(Funnel(self.op(), rhs.op(), s.op()));
            Some(())
        }
    }

    pub fn udivide(quo: &mut Self, rem: &mut Self, duo: &Self, div: &Self) -> Option<()> {
        quo.update(UQuoAssign(quo.op(), duo.op(), div.op()));
        rem.update(URemAssign(rem.op(), duo.op(), div.op()));
        Some(())
    }

    pub fn idivide(quo: &mut Self, rem: &mut Self, duo: &Self, div: &Self) -> Option<()> {
        quo.update(IQuoAssign(quo.op(), duo.op(), div.op()));
        rem.update(IRemAssign(rem.op(), duo.op(), div.op()));
        Some(())
    }
}
