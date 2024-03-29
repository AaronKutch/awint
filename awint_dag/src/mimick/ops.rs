// Note: we use `impl Into<...>` heavily instead of `U: Into<...>` generics,
// because it allows arguments to be different types.

// Note: unfortunately, `impl Into<...>` can only be used for the totally
// copyable types, and does not work for `impl Into<&'a Bits>` which causes
// failures with `&mut Bits`, same goes for any `impl ...` strategy because the
// mutable reference is moved into the function and can't be copied on the
// outside

use std::{cmp::min, marker::PhantomData, num::NonZeroUsize, ops::Range};

use awint_ext::{awi, awint_internals::USIZE_BITS, bw};
use smallvec::smallvec;
use Op::*;

use crate::{
    dag,
    mimick::{Bits, InlAwi, None, Option, Some},
    ConcatFieldsType, ConcatType, Lineage, Op,
};

// TODO there's no telling how long Try will be unstable
macro_rules! try_option {
    ($expr:expr) => {
        match $expr {
            $crate::mimick::Option::None => return None,
            $crate::mimick::Option::Some(val) => val,
            $crate::mimick::Option::Opaque(_) => unreachable!(),
        }
    };
}

macro_rules! unary {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            pub fn $fn_name(&mut self) {
                self.update_state(self.state_nzbw(), $enum_var([self.state()])).unwrap_at_runtime();
            }
        )*
    };
}

macro_rules! binary {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            #[must_use]
            pub fn $fn_name(&mut self, rhs: &Self) -> Option<()> {
                self.update_state(
                    self.state_nzbw(),
                    $enum_var([self.state(), rhs.state()])
                )
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
                    ).unwrap_at_runtime();
                }
            }

            #[must_use]
            pub fn $to_name(&self) -> dag::$prim {
                if self.state_nzbw() == dag::$prim::get_nzbw() {
                    dag::$prim::from_state(
                        self.state(),
                    )
                } else {
                    dag::$prim::new_eager_eval(
                        ZeroResize([self.state()]),
                    ).unwrap()
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
                    ).unwrap_at_runtime();
                }
            }

            #[must_use]
            pub fn $to_name(&self) -> dag::$prim {
                if self.state_nzbw() == dag::$prim::get_nzbw() {
                    dag::$prim::from_state(
                        self.state(),
                    )
                } else {
                    dag::$prim::new_eager_eval(
                        SignResize([self.state()]),
                    ).unwrap()
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
                dag::bool::new_eager_eval($enum_var([self.state()])).unwrap()
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
                    dag::bool::new_eager_eval($enum_var([self.state(), rhs.state()]))
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
                    dag::bool::new_eager_eval($enum_var([rhs.state(), self.state()]))
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
                let s = s.into();
                try_option!(self.update_state(
                    self.state_nzbw(),
                    $enum_var([self.state(), s.state()])
                ));
                Bits::efficient_ule(s, self.bw() - 1)
            }
        )*
    };
}

macro_rules! ref_self_output_usize {
    ($($fn_name:ident $enum_var:ident),*,) => {
        $(
            #[must_use]
            pub fn $fn_name(&self) -> dag::usize {
                dag::usize::new_eager_eval($enum_var([self.state()])).unwrap()
            }
        )*
    };
}

/// # Note
///
/// These functions are all mimicks of functions for [awint_ext::Bits], except
/// for the special `opaque_`.
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

    /// Assigns with the special `Op::Opaque` state, with the state of `self`
    /// being the first argument
    pub fn opaque_(&mut self, name: &'static str, with: &[&Bits]) {
        let mut v = smallvec![self.state()];
        for x in with {
            v.push(x.state());
        }
        self.update_state(self.state_nzbw(), Opaque(v, awi::Some(name)))
            .unwrap_at_runtime();
    }

    pub fn zero_(&mut self) {
        self.update_state(self.state_nzbw(), Op::Literal(awi::Awi::zero(self.nzbw())))
            .unwrap_at_runtime();
    }

    pub fn umax_(&mut self) {
        self.update_state(self.state_nzbw(), Op::Literal(awi::Awi::umax(self.nzbw())))
            .unwrap_at_runtime();
    }

    pub fn imax_(&mut self) {
        self.update_state(self.state_nzbw(), Op::Literal(awi::Awi::imax(self.nzbw())))
            .unwrap_at_runtime();
    }

    pub fn imin_(&mut self) {
        self.update_state(self.state_nzbw(), Op::Literal(awi::Awi::imin(self.nzbw())))
            .unwrap_at_runtime();
    }

    pub fn uone_(&mut self) {
        self.update_state(self.state_nzbw(), Op::Literal(awi::Awi::uone(self.nzbw())))
            .unwrap_at_runtime();
    }

    /// Static fielding given `awi::usize`s
    #[doc(hidden)]
    pub fn static_field(
        lhs: &Bits,
        to: usize,
        rhs: &Bits,
        from: usize,
        width: usize,
    ) -> awi::Option<dag::Awi> {
        use awi::*;
        if (width > lhs.bw())
            || (width > rhs.bw())
            || (to > (lhs.bw() - width))
            || (from > (rhs.bw() - width))
        {
            return None;
        }
        let res = if let Some(width) = NonZeroUsize::new(width) {
            if let Some(lhs_rem_lo) = NonZeroUsize::new(to) {
                if let Some(lhs_rem_hi) = NonZeroUsize::new(from) {
                    dag::Awi::new(
                        lhs.nzbw(),
                        Op::ConcatFields(ConcatFieldsType::from_iter([
                            (lhs.state(), 0usize, lhs_rem_lo),
                            (rhs.state(), from, width),
                            (lhs.state(), to + width.get(), lhs_rem_hi),
                        ])),
                    )
                } else {
                    dag::Awi::new(
                        lhs.nzbw(),
                        Op::ConcatFields(ConcatFieldsType::from_iter([
                            (lhs.state(), 0usize, lhs_rem_lo),
                            (rhs.state(), from, width),
                        ])),
                    )
                }
            } else if let Some(lhs_rem_hi) = NonZeroUsize::new(lhs.bw() - width.get()) {
                dag::Awi::new(
                    lhs.nzbw(),
                    Op::ConcatFields(ConcatFieldsType::from_iter([
                        (rhs.state(), from, width),
                        (lhs.state(), width.get(), lhs_rem_hi),
                    ])),
                )
            } else {
                dag::Awi::new(
                    lhs.nzbw(),
                    Op::ConcatFields(ConcatFieldsType::from_iter([(rhs.state(), from, width)])),
                )
            }
        } else {
            dag::Awi::from_bits(lhs)
        };
        Some(res)
    }

    /// Given a value `max`, this returns the number of nontrivial bits that may
    /// not be zero when the value is at most `max`
    #[doc(hidden)]
    pub fn nontrivial_bits(max: usize) -> awi::Option<NonZeroUsize> {
        NonZeroUsize::new(
            usize::try_from(max.next_power_of_two().trailing_zeros())
                .unwrap()
                .checked_add(if max.is_power_of_two() { 1 } else { 0 })
                .unwrap(),
        )
    }

    /// Given a value `s` that should not be greater than `max`, this will
    /// efficiently return `None` if it is.
    #[doc(hidden)]
    pub fn efficient_ule(s: dag::usize, max: usize) -> Option<()> {
        if let awi::Some(s) = s.state().try_get_as_usize() {
            if s <= max {
                Some(())
            } else {
                None
            }
        } else if max == 0 {
            let s_awi = dag::Awi::from_usize(s);
            let success = s_awi.is_zero();
            Option::some_at_dagtime((), success)
        } else if max >= (isize::MAX as usize) {
            let s_awi = dag::Awi::from_usize(s);
            let max_awi = dag::Awi::from_usize(max);
            let success = s_awi.ule(&max_awi).unwrap();
            Option::some_at_dagtime((), success)
        } else {
            // break up into two parts, one that should always be zero and one that either
            // doesn't need any checks or needs a small `ule` check
            let max_width_w = Bits::nontrivial_bits(max).unwrap();
            let should_be_zero_w = NonZeroUsize::new(USIZE_BITS - max_width_w.get()).unwrap();
            let should_be_zero = dag::Awi::new(
                should_be_zero_w,
                Op::ConcatFields(ConcatFieldsType::from_iter([(
                    s.state(),
                    max_width_w.get(),
                    should_be_zero_w,
                )])),
            );

            let success = if max.checked_add(1).unwrap().is_power_of_two() {
                // the bits can be whatever they want
                should_be_zero.is_zero()
            } else {
                // avoid a `USIZE_BITS` comparison
                let s_small = dag::Awi::new(
                    max_width_w,
                    Op::ConcatFields(ConcatFieldsType::from_iter([(s.state(), 0, max_width_w)])),
                );
                let mut max_small = dag::Awi::zero(max_width_w);
                max_small.usize_(max);
                should_be_zero.is_zero() & s_small.ule(&max_small).unwrap()
            };
            Option::some_at_dagtime((), success)
        }
    }

    /// Given a values `a` and `b` whose sum should not be greater than `max`,
    /// this will efficiently return `None` if it is.
    #[doc(hidden)]
    pub fn efficient_add_then_ule(a: dag::usize, b: dag::usize, max: usize) -> Option<()> {
        if let awi::Some(a) = a.state().try_get_as_usize() {
            if a > max {
                return None
            }
            Bits::efficient_ule(b, max - a)
        } else if let awi::Some(b) = b.state().try_get_as_usize() {
            if b > max {
                return None
            }
            Bits::efficient_ule(a, max - b)
        } else if max == 0 {
            let a_awi = dag::Awi::from_usize(a);
            let b_awi = dag::Awi::from_usize(b);
            let success = a_awi.is_zero() & b_awi.is_zero();
            Option::some_at_dagtime((), success)
        } else if max >= (isize::MAX as usize) {
            panic!()
        } else {
            let max_width_w = Bits::nontrivial_bits(max).unwrap();
            let should_be_zero_w = NonZeroUsize::new(USIZE_BITS - max_width_w.get()).unwrap();
            let should_be_zero_a = dag::Awi::new(
                should_be_zero_w,
                Op::ConcatFields(ConcatFieldsType::from_iter([(
                    a.state(),
                    max_width_w.get(),
                    should_be_zero_w,
                )])),
            );
            let should_be_zero_b = dag::Awi::new(
                should_be_zero_w,
                Op::ConcatFields(ConcatFieldsType::from_iter([(
                    b.state(),
                    max_width_w.get(),
                    should_be_zero_w,
                )])),
            );
            let small_a = dag::Awi::new(
                max_width_w,
                Op::ConcatFields(ConcatFieldsType::from_iter([(a.state(), 0, max_width_w)])),
            );
            let small_b = dag::Awi::new(
                max_width_w,
                Op::ConcatFields(ConcatFieldsType::from_iter([(b.state(), 0, max_width_w)])),
            );
            // avoid a `USIZE_BITS` addition and comparison
            let mut sum = dag::Awi::zero(max_width_w);
            let o = sum.cin_sum_(false, &small_a, &small_b).unwrap().0;

            let success = if max.checked_add(1).unwrap().is_power_of_two() {
                // the bits can be whatever they want
                should_be_zero_a.is_zero() & should_be_zero_b.is_zero() & (!o)
            } else {
                let mut max_small = dag::Awi::zero(max_width_w);
                max_small.usize_(max);
                should_be_zero_a.is_zero()
                    & should_be_zero_b.is_zero()
                    & sum.ule(&max_small).unwrap()
                    & (!o)
            };
            Option::some_at_dagtime((), success)
        }
    }

    #[must_use]
    pub fn mux_(&mut self, rhs: &Self, b: impl Into<dag::bool>) -> Option<()> {
        self.update_state(
            self.state_nzbw(),
            Mux([self.state(), rhs.state(), b.into().state()]),
        )
    }

    #[must_use]
    pub fn lut_(&mut self, lut: &Self, inx: &Self) -> Option<()> {
        let mut res = false;
        let lhs_w = self.state_nzbw();
        let inx_w = inx.state_nzbw();
        let lut_w = lut.state_nzbw();
        if inx_w.get() < USIZE_BITS {
            if let awi::Some(lut_len) = (1usize << inx_w.get()).checked_mul(lhs_w.get()) {
                if lut_len == lut_w.get() {
                    res = true;
                }
            }
        }
        if !res {
            return None
        }
        if let awi::Some(lut) = lut.state().try_get_as_awi() {
            // optimization for meta lowering
            self.update_state(lhs_w, StaticLut(ConcatType::from_iter([inx.state()]), lut))
        } else {
            self.update_state(self.state_nzbw(), Lut([lut.state(), inx.state()]))
        }
    }

    #[must_use]
    pub fn lut_set(&mut self, entry: &Self, inx: &Self) -> Option<()> {
        self.update_state(
            self.state_nzbw(),
            LutSet([self.state(), entry.state(), inx.state()]),
        )
    }

    #[must_use]
    pub fn get(&self, inx: impl Into<dag::usize>) -> Option<dag::bool> {
        let inx = inx.into();
        if let awi::Some(inx) = inx.state().try_get_as_usize() {
            // optimization for the meta lowering
            if inx >= self.bw() {
                None
            } else if self.bw() == 1 {
                // single bit copy
                Some(dag::bool::from_state(self.state()))
            } else {
                dag::bool::new_eager_eval(StaticGet([self.state()], inx))
            }
        } else {
            let b = dag::bool::new_eager_eval(Get([self.state(), inx.state()]));
            match b {
                None => None,
                Some(b) => {
                    Option::some_at_dagtime(b, Bits::efficient_ule(inx, self.bw() - 1).is_some())
                }
                _ => unreachable!(),
            }
        }
    }

    #[must_use]
    pub fn set(&mut self, inx: impl Into<dag::usize>, bit: impl Into<dag::bool>) -> Option<()> {
        let inx = inx.into();
        let bit = bit.into();
        let bits_w = self.state_nzbw();
        if let awi::Some(inx) = inx.state().try_get_as_usize() {
            // optimization for the meta lowering
            if inx >= self.bw() {
                None
            } else if let awi::Some(lo_rem) = NonZeroUsize::new(inx) {
                if let awi::Some(hi_rem) = NonZeroUsize::new(bits_w.get() - 1 - inx) {
                    self.update_state(
                        bits_w,
                        ConcatFields(ConcatFieldsType::from_iter([
                            (self.state(), 0, lo_rem),
                            (bit.state(), 0, bw(1)),
                            (self.state(), inx + 1, hi_rem),
                        ])),
                    )
                    .unwrap_at_runtime();
                } else {
                    // setting the last bit
                    self.update_state(
                        bits_w,
                        ConcatFields(ConcatFieldsType::from_iter([
                            (self.state(), 0, lo_rem),
                            (bit.state(), 0, bw(1)),
                        ])),
                    )
                    .unwrap_at_runtime();
                }
                Some(())
            } else if let awi::Some(rem) = NonZeroUsize::new(bits_w.get() - 1) {
                // setting the first bit
                self.update_state(
                    bits_w,
                    ConcatFields(ConcatFieldsType::from_iter([
                        (bit.state(), 0, bw(1)),
                        (self.state(), 1, rem),
                    ])),
                )
                .unwrap_at_runtime();
                Some(())
            } else {
                // setting a single bit
                self.set_state(bit.state());
                Some(())
            }
        } else {
            self.update_state(bits_w, Set([self.state(), inx.state(), bit.state()]))
                .unwrap_at_runtime();
            Bits::efficient_ule(inx, self.bw() - 1)
        }
    }

    #[must_use]
    pub fn field(
        &mut self,
        to: impl Into<dag::usize>,
        rhs: &Self,
        from: impl Into<dag::usize>,
        width: impl Into<dag::usize>,
    ) -> Option<()> {
        let to = to.into();
        let from = from.into();
        let width = width.into();
        try_option!(self.update_state(
            self.state_nzbw(),
            Field([
                self.state(),
                to.state(),
                rhs.state(),
                from.state(),
                width.state(),
            ]),
        ));
        Option::some_at_dagtime(
            (),
            Bits::efficient_add_then_ule(to, width, self.bw()).is_some()
                & Bits::efficient_add_then_ule(from, width, rhs.bw()).is_some(),
        )
    }

    #[must_use]
    pub fn field_to(
        &mut self,
        to: impl Into<dag::usize>,
        rhs: &Self,
        width: impl Into<dag::usize>,
    ) -> Option<()> {
        let to = to.into();
        let width = width.into();
        try_option!(self.update_state(
            self.state_nzbw(),
            FieldTo([self.state(), to.state(), rhs.state(), width.state()]),
        ));
        Option::some_at_dagtime(
            (),
            Bits::efficient_add_then_ule(to, width, self.bw()).is_some()
                & Bits::efficient_ule(width, rhs.bw()).is_some(),
        )
    }

    #[must_use]
    pub fn field_from(
        &mut self,
        rhs: &Self,
        from: impl Into<dag::usize>,
        width: impl Into<dag::usize>,
    ) -> Option<()> {
        let from = from.into();
        let width = width.into();
        try_option!(self.update_state(
            self.state_nzbw(),
            FieldFrom([self.state(), rhs.state(), from.state(), width.state()]),
        ));
        Option::some_at_dagtime(
            (),
            Bits::efficient_ule(width, self.bw()).is_some()
                & Bits::efficient_add_then_ule(from, width, rhs.bw()).is_some(),
        )
    }

    #[must_use]
    pub fn field_width(&mut self, rhs: &Self, width: impl Into<dag::usize>) -> Option<()> {
        let width = width.into();
        try_option!(self.update_state(
            self.state_nzbw(),
            FieldWidth([self.state(), rhs.state(), width.state()]),
        ));
        Bits::efficient_ule(width, min(self.bw(), rhs.bw()))
    }

    #[must_use]
    pub fn field_bit(
        &mut self,
        to: impl Into<dag::usize>,
        rhs: &Self,
        from: impl Into<dag::usize>,
    ) -> Option<()> {
        let to = to.into();
        let from = from.into();
        try_option!(self.update_state(
            self.state_nzbw(),
            FieldBit([self.state(), to.state(), rhs.state(), from.state()]),
        ));
        Option::some_at_dagtime(
            (),
            Bits::efficient_ule(to, self.bw().wrapping_sub(1)).is_some()
                & Bits::efficient_ule(from, rhs.bw().wrapping_sub(1)).is_some(),
        )
    }

    pub fn repeat_(&mut self, rhs: &Self) {
        self.update_state(self.state_nzbw(), Repeat([rhs.state()]))
            .unwrap_at_runtime();
    }

    pub fn resize_(&mut self, rhs: &Self, extension: impl Into<dag::bool>) {
        self.update_state(
            self.state_nzbw(),
            Resize([rhs.state(), extension.into().state()]),
        )
        .unwrap_at_runtime();
    }

    pub fn zero_resize_(&mut self, rhs: &Self) -> dag::bool {
        let b = dag::bool::new_eager_eval(ZeroResizeOverflow([rhs.state()], self.nzbw())).unwrap();
        self.update_state(self.state_nzbw(), ZeroResize([rhs.state()]))
            .unwrap_at_runtime();
        b
    }

    pub fn sign_resize_(&mut self, rhs: &Self) -> dag::bool {
        let b = dag::bool::new_eager_eval(SignResizeOverflow([rhs.state()], self.nzbw())).unwrap();
        self.update_state(self.state_nzbw(), SignResize([rhs.state()]))
            .unwrap_at_runtime();
        b
    }

    #[must_use]
    pub fn funnel_(&mut self, rhs: &Self, s: &Self) -> Option<()> {
        self.update_state(self.state_nzbw(), Funnel([rhs.state(), s.state()]))
    }

    #[must_use]
    pub fn range_or_(&mut self, range: Range<impl Into<dag::usize>>) -> Option<()> {
        let start = range.start.into();
        let end = range.end.into();
        self.update_state(
            self.state_nzbw(),
            RangeOr([self.state(), start.state(), end.state()]),
        )
    }

    #[must_use]
    pub fn range_and_(&mut self, range: Range<impl Into<dag::usize>>) -> Option<()> {
        let start = range.start.into();
        let end = range.end.into();
        self.update_state(
            self.state_nzbw(),
            RangeAnd([self.state(), start.state(), end.state()]),
        )
    }

    #[must_use]
    pub fn range_xor_(&mut self, range: Range<impl Into<dag::usize>>) -> Option<()> {
        let start = range.start.into();
        let end = range.end.into();
        self.update_state(
            self.state_nzbw(),
            RangeXor([self.state(), start.state(), end.state()]),
        )
    }

    #[must_use]
    pub fn udivide(quo: &mut Self, rem: &mut Self, duo: &Self, div: &Self) -> Option<()> {
        if (quo.bw() == rem.bw()) && (duo.bw() == div.bw()) && (quo.bw() == duo.bw()) {
            try_option!(quo.update_state(quo.state_nzbw(), UQuo([duo.state(), div.state()])));
            try_option!(rem.update_state(rem.state_nzbw(), URem([duo.state(), div.state()])));
            Option::some_at_dagtime((), !div.is_zero())
        } else {
            None
        }
    }

    #[must_use]
    #[allow(clippy::needless_pass_by_ref_mut)]
    pub fn idivide(quo: &mut Self, rem: &mut Self, duo: &mut Self, div: &mut Self) -> Option<()> {
        if (quo.bw() == rem.bw()) && (duo.bw() == div.bw()) && (quo.bw() == duo.bw()) {
            try_option!(quo.update_state(quo.state_nzbw(), IQuo([duo.state(), div.state()])));
            try_option!(rem.update_state(rem.state_nzbw(), IRem([duo.state(), div.state()])));
            Option::some_at_dagtime((), !div.is_zero())
        } else {
            None
        }
    }

    #[must_use]
    pub fn mul_add_(&mut self, lhs: &Self, rhs: &Self) -> Option<()> {
        if (self.bw() != lhs.bw()) || (self.bw() != rhs.bw()) {
            None
        } else {
            self.update_state(
                self.state_nzbw(),
                ArbMulAdd([self.state(), lhs.state(), rhs.state()]),
            )
        }
    }

    pub fn arb_umul_add_(&mut self, lhs: &Bits, rhs: &Bits) {
        self.update_state(
            self.state_nzbw(),
            ArbMulAdd([self.state(), lhs.state(), rhs.state()]),
        )
        .unwrap_at_runtime();
    }

    #[allow(clippy::needless_pass_by_ref_mut)]
    pub fn arb_imul_add_(&mut self, lhs: &mut Bits, rhs: &mut Bits) {
        let mut lhs = dag::Awi::from_bits(lhs);
        let mut rhs = dag::Awi::from_bits(rhs);
        let lhs_msb = lhs.msb();
        let rhs_msb = rhs.msb();
        lhs.neg_(lhs_msb);
        rhs.neg_(rhs_msb);
        self.neg_(lhs_msb);
        self.neg_(rhs_msb);
        self.update_state(
            self.state_nzbw(),
            ArbMulAdd([self.state(), lhs.state(), rhs.state()]),
        )
        .unwrap_at_runtime();
        self.neg_(lhs_msb);
        self.neg_(rhs_msb);
    }

    pub fn inc_(&mut self, cin: impl Into<dag::bool>) -> dag::bool {
        let b = cin.into();
        let out = dag::bool::new_eager_eval(IncCout([self.state(), b.state()])).unwrap();
        self.update_state(self.state_nzbw(), Inc([self.state(), b.state()]))
            .unwrap_at_runtime();
        out
    }

    pub fn dec_(&mut self, cin: impl Into<dag::bool>) -> dag::bool {
        let b = cin.into();
        let out = dag::bool::new_eager_eval(DecCout([self.state(), b.state()])).unwrap();
        self.update_state(self.state_nzbw(), Dec([self.state(), b.state()]))
            .unwrap_at_runtime();
        out
    }

    pub fn neg_(&mut self, neg: impl Into<dag::bool>) {
        let b = neg.into();
        self.update_state(self.state_nzbw(), Neg([self.state(), b.state()]))
            .unwrap_at_runtime();
    }

    #[must_use]
    pub fn cin_sum_(
        &mut self,
        cin: impl Into<dag::bool>,
        lhs: &Self,
        rhs: &Self,
    ) -> Option<(dag::bool, dag::bool)> {
        if (self.bw() == lhs.bw()) && (self.bw() == rhs.bw()) {
            let b = cin.into();
            let out = Some((
                dag::bool::new_eager_eval(UnsignedOverflow([b.state(), lhs.state(), rhs.state()]))
                    .unwrap(),
                dag::bool::new_eager_eval(SignedOverflow([b.state(), lhs.state(), rhs.state()]))
                    .unwrap(),
            ));
            self.update_state(
                self.state_nzbw(),
                CinSum([b.state(), lhs.state(), rhs.state()]),
            )
            .unwrap_at_runtime();
            out
        } else {
            None
        }
    }
}

#[doc(hidden)]
pub struct CCResult<T> {
    run_fielding: awi::bool,
    success: dag::bool,
    _phantom_data: PhantomData<fn() -> T>,
}

impl<T> CCResult<T> {
    pub const fn run_fielding(&self) -> bool {
        self.run_fielding
    }

    pub const fn wrap(self, t: T) -> dag::Option<T> {
        Option::Opaque(crate::mimick::option::OpaqueInternal {
            is_some: self.success,
            t: awi::Some(t),
        })
    }

    pub const fn wrap_none(self) -> Option<T> {
        None
    }
}

impl CCResult<()> {
    pub const fn wrap_if_success(self) -> Option<()> {
        Option::Opaque(crate::mimick::option::OpaqueInternal {
            is_some: self.success,
            t: awi::Some(()),
        })
    }
}

const PANIC_MSG: &str = "statically known bitwidths are needed for certain concatenation macro \
                         usages with mimicking types from `awint_dag`";

#[doc(hidden)]
impl Bits {
    #[must_use]
    pub const fn must_use<T>(t: T) -> T {
        t
    }

    pub fn usize_cast(x: impl Into<dag::usize>) -> dag::usize {
        x.into()
    }

    pub fn usize_add(lhs: impl Into<dag::usize>, rhs: impl Into<dag::usize>) -> dag::usize {
        lhs.into().wrapping_add(rhs.into())
    }

    pub fn usize_sub(lhs: impl Into<dag::usize>, rhs: impl Into<dag::usize>) -> dag::usize {
        lhs.into().wrapping_sub(rhs.into())
    }

    pub const fn unstable_raw_digits(w: usize) -> usize {
        awint_ext::awint_internals::total_digits(awint_ext::awint_internals::bw(w)).get()
    }

    pub fn unstable_max<const N: usize>(x: [impl Into<dag::usize>; N]) -> awi::usize {
        let x: Box<[_]> = Box::from(x);
        let mut x: Vec<_> = Vec::from(x);
        let last = x.pop().unwrap().into();
        let mut max = if let Op::Literal(ref lit) = last.state().get_op() {
            assert_eq!(lit.bw(), USIZE_BITS);
            lit.to_usize()
        } else {
            panic!("{}", PANIC_MSG);
        };
        for _ in 1..N {
            let last = x.pop().unwrap().into();
            if let Op::Literal(ref lit) = last.state().get_op() {
                assert_eq!(lit.bw(), USIZE_BITS);
                let val = lit.to_usize();
                if val > max {
                    max = val;
                }
            } else {
                panic!("{}", PANIC_MSG);
            }
        }
        max
    }

    // TODO use specializations like `Bits::efficient_*` for known `cw`, but we
    // definitely need better tests for this, probably need to redo the macro test
    // generator and allow `starlight` to use it
    pub fn unstable_cc_checks<const LE: usize, const GE: usize, const EQ: usize, T>(
        le0: [impl Into<dag::usize>; LE],
        le1: [impl Into<dag::usize>; LE],
        ge: [impl Into<dag::usize>; GE],
        eq: [impl Into<dag::usize>; EQ],
        cw: impl Into<dag::usize>,
        check_nonzero_cw: awi::bool,
        ok_on_zero: awi::bool,
    ) -> CCResult<T> {
        // it is just barely possible to avoid non-copy and `impl _` errors
        let le0: Box<[_]> = Box::from(le0);
        let mut le0: Vec<_> = Vec::from(le0);
        let le1: Box<[_]> = Box::from(le1);
        let mut le1: Vec<_> = Vec::from(le1);
        let ge: Box<[_]> = Box::from(ge);
        let mut ge: Vec<_> = Vec::from(ge);
        let eq: Box<[_]> = Box::from(eq);
        let mut eq: Vec<_> = Vec::from(eq);
        let cw = InlAwi::from_usize(cw.into());
        let mut b = true.into();
        for _ in 0..LE {
            b &= InlAwi::from_usize(le0.pop().unwrap().into())
                .ule(&InlAwi::from_usize(le1.pop().unwrap().into()))
                .unwrap();
        }
        for _ in 0..GE {
            b &= cw
                .uge(&InlAwi::from_usize(ge.pop().unwrap().into()))
                .unwrap();
        }
        for _ in 0..EQ {
            b &= cw
                .const_eq(&InlAwi::from_usize(eq.pop().unwrap().into()))
                .unwrap();
        }
        if check_nonzero_cw {
            if let Op::Literal(ref lit) = cw.state().get_op() {
                assert_eq!(lit.bw(), USIZE_BITS);
                if lit.to_usize() == 0 {
                    return CCResult {
                        run_fielding: false,
                        success: ok_on_zero.into(),
                        _phantom_data: PhantomData,
                    }
                }
            } else if !ok_on_zero {
                b &= !cw.is_zero();
            }
        }
        CCResult {
            run_fielding: true,
            success: b,
            _phantom_data: PhantomData,
        }
    }
}
