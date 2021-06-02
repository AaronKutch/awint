use core::num::NonZeroUsize;

use awint_internals::BITS;
use triple_arena::{Arena, TriPtr};
use Op::*;

use crate::{primitive as prim, DagBool, Lineage, Op};

#[derive(Debug)]
pub struct Bits {
    bw: NonZeroUsize,
    state: TriPtr,
    ops: Arena<Op>,
}

impl Bits {
    /// Initializes a new `Bits` with bitwidth `bw` and initial `Op` `init_op`.
    pub fn new(bw: NonZeroUsize, init_op: Op) -> Self {
        let mut a = Arena::new();
        Self {
            bw,
            state: a.insert(init_op),
            ops: a,
        }
    }
}

impl Lineage for Bits {
    fn state(&self) -> TriPtr {
        self.state
    }

    fn ops(&self) -> &Arena<Op> {
        &self.ops
    }

    fn nzbw(&self) -> NonZeroUsize {
        self.bw
    }
}

macro_rules! nullary {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&mut self) {
                self.state = self.ops.insert($enum_var);
            }
        )*
    };
}

macro_rules! unary {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&mut self) {
                self.state = self.ops.insert($enum_var(self.state()));
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
                    self.state = self.ops.insert($enum_var(self.state(), rhs.state()));
                    Some(())
                } else {
                    // unequal bitwidths results in no changes
                    None
                }
            }
        )*
    };
}

macro_rules! ref_self_output_bool {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&self) -> prim::bool {
                prim::bool::new($enum_var(self.state()))
            }
        )*
    };
}

macro_rules! compare {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&self, rhs: &Bits) -> Option<prim::bool> {
                if self.bw() == rhs.bw() {
                    Some(prim::bool::new($enum_var(self.state(), rhs.state())))
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
                prim::usize::new($enum_var(self.state()))
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

    ref_self_output_bool!(
        is_zero IsZero,
        is_umax IsUmax,
        is_imax IsImax,
        is_imin IsImin,
        is_uone IsUone,
        lsb Lsb,
        msb Msb,
        to_bool ToBool,
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
                    self.state = self.ops.insert(Lut(self.state(), lut.state(), inx.state()));
                    return Some(())
                }
            }
        }
        None
    }

    pub fn resize_assign<I>(&mut self, rhs: &Self, extension: I)
    where
        I: Into<DagBool>,
    {
        self.state = self
            .ops
            .insert(ResizeAssign(self.state(), rhs.state(), extension.into()));
    }

    pub fn funnel(&mut self, rhs: &Self, s: &Self) -> Option<()> {
        if (s.bw() >= (BITS - 1))
            || ((1usize << s.bw()) != self.bw())
            || ((self.bw() << 1) != rhs.bw())
        {
            None
        } else {
            self.state = self
                .ops
                .insert(Funnel(self.state(), rhs.state(), s.state()));
            Some(())
        }
    }

    pub fn udivide(quo: &mut Self, rem: &mut Self, duo: &Self, div: &Self) -> Option<()> {
        quo.state = quo
            .ops
            .insert(UQuoAssign(quo.state(), duo.state(), div.state()));
        rem.state = rem
            .ops
            .insert(URemAssign(rem.state(), duo.state(), div.state()));
        Some(())
    }

    pub fn idivide(quo: &mut Self, rem: &mut Self, duo: &Self, div: &Self) -> Option<()> {
        quo.state = quo
            .ops
            .insert(IQuoAssign(quo.state(), duo.state(), div.state()));
        rem.state = rem
            .ops
            .insert(IRemAssign(rem.state(), duo.state(), div.state()));
        Some(())
    }
}
