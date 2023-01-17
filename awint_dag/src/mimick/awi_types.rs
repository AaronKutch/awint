use std::{
    borrow::{Borrow, BorrowMut},
    fmt,
    marker::PhantomData,
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Index, IndexMut, RangeFull},
    rc::Rc,
};

use awint_ext::{
    awi,
    awint_internals::{assert_inlawi_invariants, bw, forward_debug_fmt},
};
use smallvec::smallvec;

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
        Self::from_state(PState::new(NonZeroUsize::new(BW).unwrap(), op, None))
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
        Self::new(Op::Opaque(smallvec![], None))
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
        awi.const_as_mut().bool_(x);
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
    ($($w:expr, $u:ident $from_u:ident $u_:ident
        $i:ident $from_i:ident $i_:ident);*;) => {
        $(
            impl InlAwi<$w, {awi::Bits::unstable_raw_digits($w)}> {
                pub fn $from_u(x: impl Into<dag::$u>) -> Self {
                    let mut awi = Self::zero();
                    awi.const_as_mut().$u_(x);
                    awi
                }

                pub fn $from_i(x: impl Into<dag::$i>) -> Self {
                    let mut awi = Self::zero();
                    awi.const_as_mut().$i_(x);
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
    8, u8 from_u8 u8_ i8 from_i8 i8_;
    16, u16 from_u16 u16_ i16 from_i16 i16_;
    32, u32 from_u32 u32_ i32 from_i32 i32_;
    64, u64 from_u64 u64_ i64 from_i64 i64_;
    128, u128 from_u128 u128_ i128 from_i128 i128_;
);

type UsizeInlAwi =
    InlAwi<{ awi::usize::BITS as usize }, { awi::Bits::unstable_raw_digits(usize::BITS as usize) }>;

impl UsizeInlAwi {
    pub fn from_usize(x: impl Into<dag::usize>) -> Self {
        let mut awi = Self::zero();
        awi.const_as_mut().usize_(x);
        awi
    }

    pub fn from_isize(x: impl Into<dag::isize>) -> Self {
        let mut awi = Self::zero();
        awi.const_as_mut().isize_(x);
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
        Self::from_state(PState::new(nzbw, op, None))
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

    pub fn opaque(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Opaque(smallvec![], None))
    }

    pub fn zero(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::ExtAwi::zero(w)))
    }

    pub fn umax(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::ExtAwi::umax(w)))
    }

    pub fn imax(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::ExtAwi::imax(w)))
    }

    pub fn imin(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::ExtAwi::imin(w)))
    }

    pub fn uone(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::ExtAwi::uone(w)))
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn panicking_opaque(w: impl Into<dag::usize>) -> Self {
        let w = w.into();
        if let Some(w) = w.state().try_get_as_usize() {
            Self::opaque(NonZeroUsize::new(w).expect("called `panicking_opaque` with zero width"))
        } else {
            panic!("Input was not evaluatable to a literal `usize`");
        }
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn panicking_zero(w: impl Into<dag::usize>) -> Self {
        let w = w.into();
        if let Some(w) = w.state().try_get_as_usize() {
            Self::zero(NonZeroUsize::new(w).expect("called `panicking_zero` with zero width"))
        } else {
            panic!("Input was not evaluatable to a literal `usize`");
        }
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn panicking_umax(w: impl Into<dag::usize>) -> Self {
        let w = w.into();
        if let Some(w) = w.state().try_get_as_usize() {
            Self::umax(NonZeroUsize::new(w).expect("called `panicking_umax` with zero width"))
        } else {
            panic!("Input was not evaluatable to a literal `usize`");
        }
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn panicking_imax(w: impl Into<dag::usize>) -> Self {
        let w = w.into();
        if let Some(w) = w.state().try_get_as_usize() {
            Self::imax(NonZeroUsize::new(w).expect("called `panicking_imax` with zero width"))
        } else {
            panic!("Input was not evaluatable to a literal `usize`");
        }
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn panicking_imin(w: impl Into<dag::usize>) -> Self {
        let w = w.into();
        if let Some(w) = w.state().try_get_as_usize() {
            Self::imin(NonZeroUsize::new(w).expect("called `panicking_imin` with zero width"))
        } else {
            panic!("Input was not evaluatable to a literal `usize`");
        }
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn panicking_uone(w: impl Into<dag::usize>) -> Self {
        let w = w.into();
        if let Some(w) = w.state().try_get_as_usize() {
            Self::uone(NonZeroUsize::new(w).expect("called `panicking_uone` with zero width"))
        } else {
            panic!("Input was not evaluatable to a literal `usize`");
        }
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
