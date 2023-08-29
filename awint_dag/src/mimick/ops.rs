// Note: we use `impl Into<...>` heavily instead of `U: Into<...>` generics,
// because it allows arguments to be different types

use std::marker::PhantomData;

use awint_ext::{awi, awint_internals::USIZE_BITS};
use smallvec::smallvec;
use Op::*;

use crate::{
    dag,
    mimick::{Bits, InlAwi, None, Option, Some},
    Lineage, Op,
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
                let s = s.into();
                try_option!(self.update_state(
                    self.state_nzbw(),
                    $enum_var([self.state(), s.state()])
                ));
                let ok = InlAwi::from_usize(s).ult(&InlAwi::from_usize(self.bw())).unwrap();
                Option::some_at_dagtime((), ok)
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
        self.update_state(
            self.state_nzbw(),
            Opaque(smallvec![self.state()], awi::None),
        )
        .unwrap_at_runtime();
    }

    pub fn opaque_with_(&mut self, with: &[&Bits], name: awi::Option<&'static str>) {
        let mut v = smallvec![self.state()];
        for x in with {
            v.push(x.state());
        }
        self.update_state(self.state_nzbw(), Opaque(v, name))
            .unwrap_at_runtime();
    }

    pub fn zero_(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awi::ExtAwi::zero(self.nzbw())),
        )
        .unwrap_at_runtime();
    }

    pub fn umax_(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awi::ExtAwi::umax(self.nzbw())),
        )
        .unwrap_at_runtime();
    }

    pub fn imax_(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awi::ExtAwi::imax(self.nzbw())),
        )
        .unwrap_at_runtime();
    }

    pub fn imin_(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awi::ExtAwi::imin(self.nzbw())),
        )
        .unwrap_at_runtime();
    }

    pub fn uone_(&mut self) {
        self.update_state(
            self.state_nzbw(),
            Op::Literal(awi::ExtAwi::uone(self.nzbw())),
        )
        .unwrap_at_runtime();
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
        self.update_state(self.state_nzbw(), Lut([lut.state(), inx.state()]))
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
            } else {
                Some(dag::bool::new(StaticGet([self.state()], inx)))
            }
        } else {
            let ok = InlAwi::from_usize(inx)
                .ult(&InlAwi::from_usize(self.bw()))
                .unwrap();
            Option::some_at_dagtime(dag::bool::new(Get([self.state(), inx.state()])), ok)
        }
    }

    #[must_use]
    pub fn set(&mut self, inx: impl Into<dag::usize>, bit: impl Into<dag::bool>) -> Option<()> {
        let inx = inx.into();
        let bit = bit.into();
        if let awi::Some(inx) = inx.state().try_get_as_usize() {
            // optimization for the meta lowering
            if inx >= self.bw() {
                None
            } else {
                self.update_state(
                    self.state_nzbw(),
                    StaticSet([self.state(), bit.state()], inx),
                )
                .unwrap_at_runtime();
                Some(())
            }
        } else {
            self.update_state(
                self.state_nzbw(),
                Set([self.state(), inx.state(), bit.state()]),
            )
            .unwrap_at_runtime();
            let ok = InlAwi::from_usize(inx)
                .ult(&InlAwi::from_usize(self.bw()))
                .unwrap();
            Option::some_at_dagtime((), ok)
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
        let to = InlAwi::from_usize(to);
        let from = InlAwi::from_usize(from);
        let width = InlAwi::from_usize(width);
        let mut tmp0 = InlAwi::from_usize(self.bw());
        tmp0.sub_(&width).unwrap();
        let mut tmp1 = InlAwi::from_usize(rhs.bw());
        tmp1.sub_(&width).unwrap();
        let ok = width.ule(&InlAwi::from_usize(self.bw())).unwrap()
            & width.ule(&InlAwi::from_usize(rhs.bw())).unwrap()
            & to.ule(&tmp0).unwrap()
            & from.ule(&tmp1).unwrap();
        Option::some_at_dagtime((), ok)
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
        let to = InlAwi::from_usize(to);
        let width = InlAwi::from_usize(width);
        let mut tmp = InlAwi::from_usize(self.bw());
        tmp.sub_(&width).unwrap();
        let ok = width.ule(&InlAwi::from_usize(self.bw())).unwrap()
            & width.ule(&InlAwi::from_usize(rhs.bw())).unwrap()
            & to.ule(&tmp).unwrap();
        Option::some_at_dagtime((), ok)
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
        let from = InlAwi::from_usize(from);
        let width = InlAwi::from_usize(width);
        let mut tmp = InlAwi::from_usize(rhs.bw());
        tmp.sub_(&width).unwrap();
        let ok = width.ule(&InlAwi::from_usize(self.bw())).unwrap()
            & width.ule(&InlAwi::from_usize(rhs.bw())).unwrap()
            & from.ule(&tmp).unwrap();
        Option::some_at_dagtime((), ok)
    }

    #[must_use]
    pub fn field_width(&mut self, rhs: &Self, width: impl Into<dag::usize>) -> Option<()> {
        let width = width.into();
        try_option!(self.update_state(
            self.state_nzbw(),
            FieldWidth([self.state(), rhs.state(), width.state()]),
        ));
        let width = InlAwi::from_usize(width);
        let ok = width.ule(&InlAwi::from_usize(self.bw())).unwrap()
            & width.ule(&InlAwi::from_usize(rhs.bw())).unwrap();
        Option::some_at_dagtime((), ok)
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
        let to = InlAwi::from_usize(to);
        let from = InlAwi::from_usize(from);
        let ok = to.ult(&InlAwi::from_usize(self.bw())).unwrap()
            & from.ult(&InlAwi::from_usize(rhs.bw())).unwrap();
        Option::some_at_dagtime((), ok)
    }

    pub fn resize_(&mut self, rhs: &Self, extension: impl Into<dag::bool>) {
        self.update_state(
            self.state_nzbw(),
            Resize([rhs.state(), extension.into().state()]),
        )
        .unwrap_at_runtime();
    }

    pub fn zero_resize_(&mut self, rhs: &Self) -> dag::bool {
        let b = dag::bool::new(ZeroResizeOverflow([rhs.state()], self.nzbw()));
        self.update_state(self.state_nzbw(), ZeroResize([rhs.state()]))
            .unwrap_at_runtime();
        b
    }

    pub fn sign_resize_(&mut self, rhs: &Self) -> dag::bool {
        let b = dag::bool::new(SignResizeOverflow([rhs.state()], self.nzbw()));
        self.update_state(self.state_nzbw(), SignResize([rhs.state()]))
            .unwrap_at_runtime();
        b
    }

    #[must_use]
    pub fn funnel_(&mut self, rhs: &Self, s: &Self) -> Option<()> {
        self.update_state(self.state_nzbw(), Funnel([rhs.state(), s.state()]))
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
            ArbMulAdd([self.state(), lhs.state(), rhs.state()]),
        )
        .unwrap_at_runtime();
        self.neg_(lhs_msb);
        self.neg_(rhs_msb);
    }

    pub fn inc_(&mut self, cin: impl Into<dag::bool>) -> dag::bool {
        let b = cin.into();
        let out = dag::bool::new(IncCout([self.state(), b.state()]));
        self.update_state(self.state_nzbw(), Inc([self.state(), b.state()]))
            .unwrap_at_runtime();
        out
    }

    pub fn dec_(&mut self, cin: impl Into<dag::bool>) -> dag::bool {
        let b = cin.into();
        let out = dag::bool::new(DecCout([self.state(), b.state()]));
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
                dag::bool::new(UnsignedOverflow([b.state(), lhs.state(), rhs.state()])),
                dag::bool::new(SignedOverflow([b.state(), lhs.state(), rhs.state()])),
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
    _phantom_data: PhantomData<T>,
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
        let mut max = if let Op::Literal(ref lit) = last.state().cloned_state().unwrap().op {
            assert_eq!(lit.bw(), USIZE_BITS);
            lit.to_usize()
        } else {
            panic!();
        };
        for _ in 1..N {
            let last = x.pop().unwrap().into();
            if let Op::Literal(ref lit) = last.state().cloned_state().unwrap().op {
                assert_eq!(lit.bw(), USIZE_BITS);
                let val = lit.to_usize();
                if val > max {
                    max = val;
                }
            } else {
                panic!();
            }
        }
        max
    }

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
            if let Op::Literal(ref lit) = cw.state().cloned_state().unwrap().op {
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
