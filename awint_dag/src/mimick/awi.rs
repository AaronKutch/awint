use std::{
    borrow::{Borrow, BorrowMut},
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Index, IndexMut, RangeFull},
    rc::Rc,
};

use awint_internals::*;

use crate::{
    mimick::{Bits, Lineage, State},
    primitive as prim, Op,
};

/// Mimicking `awint_core::InlAwi`
#[derive(Debug)]
// Note: must use `Bits` instead of `State`, because we need to return
// references
pub struct InlAwi<const BW: usize, const LEN: usize>(Bits);

impl<const BW: usize, const LEN: usize> Lineage for InlAwi<BW, LEN> {
    fn from_state(state: Rc<State>) -> Self {
        Self(Bits::from_state(state))
    }

    fn hidden_const_nzbw() -> Option<NonZeroUsize> {
        Some(NonZeroUsize::new(BW).unwrap())
    }

    fn state(&self) -> Rc<State> {
        self.0.state()
    }
}

impl<const BW: usize, const LEN: usize> Clone for InlAwi<BW, LEN> {
    fn clone(&self) -> Self {
        Self::new(Op::Copy, vec![self.state()])
    }
}

impl<const BW: usize, const LEN: usize> InlAwi<BW, LEN> {
    fn new(op: Op, ops: Vec<Rc<State>>) -> Self {
        Self::from_state(State::new(
            Some(Self::hidden_const_nzbw().unwrap()),
            op,
            ops,
        ))
    }

    pub fn const_nzbw() -> NonZeroUsize {
        assert_inlawi_invariants::<BW, LEN>();
        NonZeroUsize::new(BW).unwrap()
    }

    pub fn const_bw() -> usize {
        assert_inlawi_invariants::<BW, LEN>();
        BW
    }

    pub fn nzbw(&self) -> NonZeroUsize {
        Self::const_nzbw()
    }

    pub fn bw(&self) -> usize {
        Self::const_bw()
    }

    pub fn const_as_ref(&self) -> &Bits {
        &self.0
    }

    pub fn const_as_mut(&mut self) -> &mut Bits {
        &mut self.0
    }

    pub fn const_raw_len() -> usize {
        assert_inlawi_invariants::<BW, LEN>();
        LEN
    }

    #[doc(hidden)]
    pub fn unstable_from_u8_slice(buf: &[u8]) -> Self {
        Self::new(
            Op::Literal(awint_ext::ExtAwi::from_bits(
                awint_core::InlAwi::<BW, LEN>::unstable_from_u8_slice(buf).const_as_ref(),
            )),
            vec![],
        )
    }

    pub fn opaque() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::Opaque, vec![])
    }

    pub fn zero() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::Literal(awint_ext::ExtAwi::zero(bw(BW))), vec![])
    }

    pub fn umax() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::Literal(awint_ext::ExtAwi::umax(bw(BW))), vec![])
    }

    pub fn imax() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::Literal(awint_ext::ExtAwi::imax(bw(BW))), vec![])
    }

    pub fn imin() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::Literal(awint_ext::ExtAwi::imin(bw(BW))), vec![])
    }

    pub fn uone() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::Literal(awint_ext::ExtAwi::uone(bw(BW))), vec![])
    }
}

impl<const BW: usize, const LEN: usize> Deref for InlAwi<BW, LEN> {
    type Target = Bits;

    fn deref(&self) -> &Self::Target {
        self.const_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> DerefMut for InlAwi<BW, LEN> {
    fn deref_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

impl<const BW: usize, const LEN: usize> Index<RangeFull> for InlAwi<BW, LEN> {
    type Output = Bits;

    fn index(&self, _i: RangeFull) -> &Bits {
        self.const_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> Borrow<Bits> for InlAwi<BW, LEN> {
    fn borrow(&self) -> &Bits {
        self.const_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> AsRef<Bits> for InlAwi<BW, LEN> {
    fn as_ref(&self) -> &Bits {
        self.const_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> IndexMut<RangeFull> for InlAwi<BW, LEN> {
    fn index_mut(&mut self, _i: RangeFull) -> &mut Bits {
        self.const_as_mut()
    }
}

impl<const BW: usize, const LEN: usize> BorrowMut<Bits> for InlAwi<BW, LEN> {
    fn borrow_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

impl<const BW: usize, const LEN: usize> AsMut<Bits> for InlAwi<BW, LEN> {
    fn as_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

macro_rules! inlawi_from {
    ($($w:expr, $u:ident $from_u:ident $u_assign:ident
        $i:ident $from_i:ident $i_assign:ident);*;) => {
        $(
            impl InlAwi<$w, {awint_core::Bits::unstable_raw_digits($w)}> {
                pub fn $from_u(x: $u) -> Self {
                    let mut awi = Self::zero();
                    awi.const_as_mut().$u_assign(x);
                    awi
                }

                pub fn $from_i(x: $i) -> Self {
                    let mut awi = Self::zero();
                    awi.const_as_mut().$i_assign(x);
                    awi
                }
            }
        )*
    };
}

inlawi_from!(
    8, u8 from_u8 u8_assign i8 from_i8 i8_assign;
    16, u16 from_u16 u16_assign i16 from_i16 i16_assign;
    32, u32 from_u32 u32_assign i32 from_i32 i32_assign;
    64, u64 from_u64 u64_assign i64 from_i64 i64_assign;
    128, u128 from_u128 u128_assign i128 from_i128 i128_assign;
);

/// Mimicking `awint_ext::ExtAwi`
#[derive(Debug)]
pub struct ExtAwi(Bits);

impl Lineage for ExtAwi {
    fn from_state(state: Rc<State>) -> Self {
        Self(Bits::from_state(state))
    }

    fn hidden_const_nzbw() -> Option<NonZeroUsize> {
        None
    }

    fn state(&self) -> Rc<State> {
        self.0.state()
    }
}

impl Clone for ExtAwi {
    fn clone(&self) -> Self {
        Self::new(self.nzbw(), Op::Copy, vec![self.state()])
    }
}

impl ExtAwi {
    fn new(nzbw: NonZeroUsize, op: Op, ops: Vec<Rc<State>>) -> Self {
        Self::from_state(State::new(Some(nzbw), op, ops))
    }

    /*
    pub fn bw(&self) -> prim::usize {
        prim::usize::new(BwAssign, vec![self.state()])
    }
    */

    pub fn nzbw(&self) -> NonZeroUsize {
        self.state().nzbw.unwrap()
    }

    pub fn bw(&self) -> usize {
        self.nzbw().get()
    }

    pub fn const_as_ref(&self) -> &Bits {
        &self.0
    }

    pub fn const_as_mut(&mut self) -> &mut Bits {
        &mut self.0
    }

    pub fn from_bits(bits: &Bits) -> ExtAwi {
        Self::new(bits.nzbw(), Op::Copy, vec![bits.state()])
    }

    pub fn opaque(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::Opaque, vec![])
    }

    pub fn zero(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::Literal(awint_ext::ExtAwi::zero(bw)), vec![])
    }

    pub fn umax(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::Literal(awint_ext::ExtAwi::umax(bw)), vec![])
    }

    pub fn imax(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::Literal(awint_ext::ExtAwi::imax(bw)), vec![])
    }

    pub fn imin(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::Literal(awint_ext::ExtAwi::imin(bw)), vec![])
    }

    pub fn uone(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::Literal(awint_ext::ExtAwi::uone(bw)), vec![])
    }

    #[doc(hidden)]
    pub fn panicking_zero(bw: usize) -> Self {
        Self::zero(NonZeroUsize::new(bw).unwrap())
    }

    #[doc(hidden)]
    pub fn panicking_umax(bw: usize) -> Self {
        Self::umax(NonZeroUsize::new(bw).unwrap())
    }

    #[doc(hidden)]
    pub fn panicking_imax(bw: usize) -> Self {
        Self::imax(NonZeroUsize::new(bw).unwrap())
    }

    #[doc(hidden)]
    pub fn panicking_imin(bw: usize) -> Self {
        Self::imin(NonZeroUsize::new(bw).unwrap())
    }

    #[doc(hidden)]
    pub fn panicking_uone(bw: usize) -> Self {
        Self::uone(NonZeroUsize::new(bw).unwrap())
    }
}

impl Deref for ExtAwi {
    type Target = Bits;

    fn deref(&self) -> &Self::Target {
        self.const_as_ref()
    }
}

impl DerefMut for ExtAwi {
    fn deref_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

impl Index<RangeFull> for ExtAwi {
    type Output = Bits;

    fn index(&self, _i: RangeFull) -> &Bits {
        self.const_as_ref()
    }
}

impl Borrow<Bits> for ExtAwi {
    fn borrow(&self) -> &Bits {
        self.const_as_ref()
    }
}

impl AsRef<Bits> for ExtAwi {
    fn as_ref(&self) -> &Bits {
        self.const_as_ref()
    }
}

impl IndexMut<RangeFull> for ExtAwi {
    fn index_mut(&mut self, _i: RangeFull) -> &mut Bits {
        self.const_as_mut()
    }
}

impl BorrowMut<Bits> for ExtAwi {
    fn borrow_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

impl AsMut<Bits> for ExtAwi {
    fn as_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

impl From<&Bits> for ExtAwi {
    fn from(bits: &Bits) -> ExtAwi {
        Self::new(bits.nzbw(), Op::Copy, vec![bits.state()])
    }
}

impl<const BW: usize, const LEN: usize> From<InlAwi<BW, LEN>> for ExtAwi {
    fn from(awi: InlAwi<BW, LEN>) -> ExtAwi {
        Self::new(awi.nzbw(), Op::Copy, vec![awi.state()])
    }
}

impl From<bool> for ExtAwi {
    fn from(x: bool) -> ExtAwi {
        Self::new(prim::bool::hidden_const_nzbw().unwrap(), Op::Copy, vec![
            prim::bool::from(x).state(),
        ])
    }
}

impl From<prim::bool> for ExtAwi {
    fn from(x: prim::bool) -> ExtAwi {
        Self::new(prim::bool::hidden_const_nzbw().unwrap(), Op::Copy, vec![
            x.state()
        ])
    }
}

macro_rules! to_extawi {
    ($($ty:ident, $assign:ident);*;) => {
        $(
            impl From<$ty> for ExtAwi {
                fn from(x: $ty) -> Self {
                    Self::new(
                        prim::$ty::hidden_const_nzbw().unwrap(),
                        Op::Copy,
                        vec![prim::$ty::from(x).state()]
                    )
                }
            }

            impl From<prim::$ty> for ExtAwi {
                fn from(x: prim::$ty) -> Self {
                    Self::new(prim::$ty::hidden_const_nzbw().unwrap(), Op::Copy, vec![x.state()])
                }
            }
        )*
    };
}

to_extawi!(
    usize, usize_assign;
    isize, isize_assign;
    u8, u8_assign;
    i8, i8_assign;
    u16, u16_assign;
    i16, i16_assign;
    u32, u32_assign;
    i32, i32_assign;
    u64, u64_assign;
    i64, i64_assign;
    u128, u128_assign;
    i128, i128_assign;
);
