use std::{
    borrow::{Borrow, BorrowMut},
    fmt,
    marker::PhantomData,
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Index, IndexMut, RangeFull},
    rc::Rc,
};

use awint_ext::awi;
use awint_internals::*;

use crate::{dag, Bits, Lineage, Op, PState};

/// Mimicking `awint_core::InlAwi`.
///
/// Note: `inlawi!(opaque: ..64)` just works
#[derive(Clone, Copy)]
pub struct InlAwi<const BW: usize, const LEN: usize> {
    // prevents the type from implementing `Send` or `Sync` on stable while still being able to be
    // `Copy`
    _no_send_or_sync: PhantomData<Rc<()>>,
    pub(in crate::mimick) _inlawi_raw: [PState; 1],
}

impl<const BW: usize, const LEN: usize> Lineage for InlAwi<BW, LEN> {
    fn state(&self) -> PState {
        self._inlawi_raw[0]
    }
}

impl<const BW: usize, const LEN: usize> InlAwi<BW, LEN> {
    pub(crate) fn from_state(state: PState) -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self {
            _inlawi_raw: [state],
            _no_send_or_sync: PhantomData,
        }
    }

    pub(crate) fn new(op: Op<PState>) -> Self {
        Self::from_state(PState::new(NonZeroUsize::new(BW).unwrap(), op))
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

    pub fn const_raw_len() -> usize {
        assert_inlawi_invariants::<BW, LEN>();
        LEN
    }

    #[doc(hidden)]
    pub fn unstable_from_u8_slice(buf: &[u8]) -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::Literal(awi::ExtAwi::from_bits(
            awi::InlAwi::<BW, LEN>::unstable_from_u8_slice(buf).const_as_ref(),
        )))
    }

    pub fn opaque() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::Opaque(vec![]))
    }

    pub fn zero() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::Literal(awi::ExtAwi::zero(bw(BW))))
    }

    pub fn umax() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::Literal(awi::ExtAwi::umax(bw(BW))))
    }

    pub fn imax() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::Literal(awi::ExtAwi::imax(bw(BW))))
    }

    pub fn imin() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::Literal(awi::ExtAwi::imin(bw(BW))))
    }

    pub fn uone() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::Literal(awi::ExtAwi::uone(bw(BW))))
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

impl<const BW: usize, const LEN: usize> fmt::Debug for InlAwi<BW, LEN> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InlAwi({:?})", self._inlawi_raw[0])
    }
}

macro_rules! forward_inlawi_fmt {
    ($($name:ident)*) => {
        $(
            impl<const BW: usize, const LEN: usize> fmt::$name for InlAwi<BW, LEN> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::Debug::fmt(self, f)
                }
            }
        )*
    };
}

forward_inlawi_fmt!(Display LowerHex UpperHex Octal Binary);

impl InlAwi<1, { awi::Bits::unstable_raw_digits(1) }> {
    pub fn from_bool(x: impl Into<dag::bool>) -> Self {
        let mut awi = Self::zero();
        awi.const_as_mut().bool_assign(x);
        awi
    }
}

impl From<dag::bool> for InlAwi<1, { awi::Bits::unstable_raw_digits(1) }> {
    fn from(x: dag::bool) -> Self {
        Self::from_bool(x)
    }
}

impl From<awi::bool> for InlAwi<1, { awi::Bits::unstable_raw_digits(1) }> {
    fn from(x: awi::bool) -> Self {
        Self::from_bool(x)
    }
}

macro_rules! inlawi_from {
    ($($w:expr, $u:ident $from_u:ident $u_assign:ident
        $i:ident $from_i:ident $i_assign:ident);*;) => {
        $(
            impl InlAwi<$w, {awi::Bits::unstable_raw_digits($w)}> {
                pub fn $from_u(x: impl Into<dag::$u>) -> Self {
                    let mut awi = Self::zero();
                    awi.const_as_mut().$u_assign(x);
                    awi
                }

                pub fn $from_i(x: impl Into<dag::$i>) -> Self {
                    let mut awi = Self::zero();
                    awi.const_as_mut().$i_assign(x);
                    awi
                }
            }

            impl From<dag::$u> for InlAwi<$w, {awi::Bits::unstable_raw_digits($w)}> {
                fn from(x: dag::$u) -> Self {
                    Self::$from_u(x)
                }
            }

            impl From<dag::$i> for InlAwi<$w, {awi::Bits::unstable_raw_digits($w)}> {
                fn from(x: dag::$i) -> Self {
                    Self::$from_i(x)
                }
            }

            impl From<awi::$u> for InlAwi<$w, {awi::Bits::unstable_raw_digits($w)}> {
                fn from(x: awi::$u) -> Self {
                    Self::$from_u(x)
                }
            }

            impl From<awi::$i> for InlAwi<$w, {awi::Bits::unstable_raw_digits($w)}> {
                fn from(x: awi::$i) -> Self {
                    Self::$from_i(x)
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

type UsizeInlAwi =
    InlAwi<{ awi::usize::BITS as usize }, { awi::Bits::unstable_raw_digits(usize::BITS as usize) }>;

impl UsizeInlAwi {
    pub fn from_usize(x: impl Into<dag::usize>) -> Self {
        let mut awi = Self::zero();
        awi.const_as_mut().usize_assign(x);
        awi
    }

    pub fn from_isize(x: impl Into<dag::isize>) -> Self {
        let mut awi = Self::zero();
        awi.const_as_mut().isize_assign(x);
        awi
    }
}

impl From<dag::usize> for UsizeInlAwi {
    fn from(x: dag::usize) -> Self {
        Self::from_usize(x)
    }
}

impl From<dag::isize> for UsizeInlAwi {
    fn from(x: dag::isize) -> Self {
        Self::from_isize(x)
    }
}

impl From<awi::usize> for UsizeInlAwi {
    fn from(x: awi::usize) -> Self {
        Self::from_usize(x)
    }
}

impl From<awi::isize> for UsizeInlAwi {
    fn from(x: awi::isize) -> Self {
        Self::from_isize(x)
    }
}

/// Mimicking `awint_ext::ExtAwi`
///
/// Note: `extawi!(opaque: ..64)` just works
#[derive(Clone)]
pub struct ExtAwi {
    _no_send_or_sync: PhantomData<Rc<()>>,
    pub(in crate::mimick) _extawi_raw: [PState; 1],
}

impl Lineage for ExtAwi {
    fn state(&self) -> PState {
        self._extawi_raw[0]
    }
}

impl ExtAwi {
    pub(crate) fn from_state(state: PState) -> Self {
        Self {
            _extawi_raw: [state],
            _no_send_or_sync: PhantomData,
        }
    }

    pub(crate) fn new(nzbw: NonZeroUsize, op: Op<PState>) -> Self {
        Self::from_state(PState::new(nzbw, op))
    }

    pub fn nzbw(&self) -> NonZeroUsize {
        self.state_nzbw()
    }

    pub fn bw(&self) -> usize {
        self.nzbw().get()
    }

    pub fn from_bits(bits: &Bits) -> ExtAwi {
        Self::from_state(bits.state())
    }

    pub fn opaque(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::Opaque(vec![]))
    }

    pub fn zero(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::Literal(awi::ExtAwi::zero(bw)))
    }

    pub fn umax(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::Literal(awi::ExtAwi::umax(bw)))
    }

    pub fn imax(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::Literal(awi::ExtAwi::imax(bw)))
    }

    pub fn imin(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::Literal(awi::ExtAwi::imin(bw)))
    }

    pub fn uone(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::Literal(awi::ExtAwi::uone(bw)))
    }

    #[doc(hidden)]
    pub fn panicking_opaque(bw: awi::usize) -> Self {
        Self::opaque(NonZeroUsize::new(bw).unwrap())
    }

    #[doc(hidden)]
    pub fn panicking_zero(bw: awi::usize) -> Self {
        Self::zero(NonZeroUsize::new(bw).unwrap())
    }

    #[doc(hidden)]
    pub fn panicking_umax(bw: awi::usize) -> Self {
        Self::umax(NonZeroUsize::new(bw).unwrap())
    }

    #[doc(hidden)]
    pub fn panicking_imax(bw: awi::usize) -> Self {
        Self::imax(NonZeroUsize::new(bw).unwrap())
    }

    #[doc(hidden)]
    pub fn panicking_imin(bw: awi::usize) -> Self {
        Self::imin(NonZeroUsize::new(bw).unwrap())
    }

    #[doc(hidden)]
    pub fn panicking_uone(bw: awi::usize) -> Self {
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

impl fmt::Debug for ExtAwi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExtAwi({:?})", self._extawi_raw[0])
    }
}

forward_debug_fmt!(ExtAwi);

impl From<&Bits> for ExtAwi {
    fn from(bits: &Bits) -> ExtAwi {
        Self::from_state(bits.state())
    }
}

impl From<&awi::Bits> for ExtAwi {
    fn from(bits: &awi::Bits) -> ExtAwi {
        Self::new(bits.nzbw(), Op::Literal(awi::ExtAwi::from(bits)))
    }
}

impl<const BW: usize, const LEN: usize> From<InlAwi<BW, LEN>> for ExtAwi {
    fn from(awi: InlAwi<BW, LEN>) -> ExtAwi {
        Self::from_state(awi.state())
    }
}

macro_rules! extawi_from {
    ($($ty:ident, $from:ident);*;) => {
        $(
            impl ExtAwi {
                pub fn $from(x: impl Into<dag::$ty>) -> Self {
                    Self::from(InlAwi::$from(x))
                }
            }

            impl From<dag::$ty> for ExtAwi {
                fn from(x: dag::$ty) -> Self {
                    Self::$from(x)
                }
            }

            impl From<awi::$ty> for ExtAwi {
                fn from(x: awi::$ty) -> Self {
                    Self::$from(x)
                }
            }
        )*
    };
}

extawi_from!(
    bool, from_bool;
    u8, from_u8;
    u16, from_u16;
    u32, from_u32;
    u64, from_u64;
    u128, from_u128;
    usize, from_usize;
    i8, from_i8;
    i16, from_i16;
    i32, from_i32;
    i64, from_i64;
    i128, from_i128;
    isize, from_isize;
);
