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
    awint_internals::{bw, forward_debug_fmt, RawStackBits, USIZE_BITS},
};
use smallvec::smallvec;

use crate::{dag, mimick::Bits, Lineage, Op, PState};

/// Mimicking [awint_ext::InlAwi]
///
/// Note: `inlawi!(opaque: ..64)` just works
#[derive(Clone, Copy)]
#[repr(C)] // needed for `internal_as_ref*`, also this needs to just be a `PState`
pub struct InlAwi<const BW: usize, const LEN: usize> {
    // prevents the type from implementing `Send` or `Sync` on stable while still being able to be
    // `Copy`
    _no_send_or_sync: PhantomData<fn() -> Rc<()>>,
    pub(in crate::mimick) _state: PState,
}

impl<const BW: usize, const LEN: usize> Lineage for InlAwi<BW, LEN> {
    fn state(&self) -> PState {
        self._state
    }
}

/// # Note
///
/// These functions are all mimicks of functions for [awint_ext::InlAwi], except
/// for the special `arg`, `opaque`, and `opaque_with`.
impl<const BW: usize, const LEN: usize> InlAwi<BW, LEN> {
    /// Special mimick-only function, most users should be using other
    /// construction methods
    pub fn from_state(state: PState) -> Self {
        RawStackBits::<BW, LEN>::_assert_invariants();
        Self {
            _no_send_or_sync: PhantomData,
            _state: state,
        }
    }

    /// Special mimick-only function, most users should be using other
    /// construction methods
    pub fn new(op: Op<PState>) -> Self {
        Self::from_state(PState::new(NonZeroUsize::new(BW).unwrap(), op, None))
    }

    pub fn const_nzbw() -> NonZeroUsize {
        RawStackBits::<BW, LEN>::_assert_invariants();
        NonZeroUsize::new(BW).unwrap()
    }

    pub fn const_bw() -> usize {
        RawStackBits::<BW, LEN>::_assert_invariants();
        BW
    }

    pub fn nzbw(&self) -> NonZeroUsize {
        Self::const_nzbw()
    }

    pub fn bw(&self) -> usize {
        Self::const_bw()
    }

    pub fn const_raw_len() -> usize {
        RawStackBits::<BW, LEN>::_assert_invariants();
        LEN
    }

    #[doc(hidden)]
    pub fn unstable_from_u8_slice(buf: &[u8]) -> Self {
        RawStackBits::<BW, LEN>::_assert_invariants();
        Self::new(Op::Literal(awi::Awi::from_bits(
            &awi::InlAwi::<BW, LEN>::unstable_from_u8_slice(buf),
        )))
    }

    /// Constructs with the special `Op::Argument` state
    pub fn arg(arg: awi::InlAwi<BW, LEN>) -> Self {
        Self::from_state(PState::new(
            Self::const_nzbw(),
            Op::Argument(awi::Awi::from(arg)),
            None,
        ))
    }

    /// Constructs with the special `Op::Opaque` state, with a `None` name and
    /// no arguments
    pub fn opaque() -> Self {
        RawStackBits::<BW, LEN>::_assert_invariants();
        Self::new(Op::Opaque(smallvec![], None))
    }

    /// Constructs with the special `Op::Opaque` state, with custom bitwidth,
    /// name, and arguments
    pub fn opaque_with(name: &'static str, with: &[&Bits]) -> Self {
        let mut v = smallvec![];
        for x in with {
            v.push(x.state());
        }
        Self::new(Op::Opaque(v, Some(name)))
    }

    pub fn zero() -> Self {
        RawStackBits::<BW, LEN>::_assert_invariants();
        Self::new(Op::Literal(awi::Awi::zero(bw(BW))))
    }

    pub fn umax() -> Self {
        RawStackBits::<BW, LEN>::_assert_invariants();
        Self::new(Op::Literal(awi::Awi::umax(bw(BW))))
    }

    pub fn imax() -> Self {
        RawStackBits::<BW, LEN>::_assert_invariants();
        Self::new(Op::Literal(awi::Awi::imax(bw(BW))))
    }

    pub fn imin() -> Self {
        RawStackBits::<BW, LEN>::_assert_invariants();
        Self::new(Op::Literal(awi::Awi::imin(bw(BW))))
    }

    pub fn uone() -> Self {
        RawStackBits::<BW, LEN>::_assert_invariants();
        Self::new(Op::Literal(awi::Awi::uone(bw(BW))))
    }
}

impl<const BW: usize, const LEN: usize> Deref for InlAwi<BW, LEN> {
    type Target = Bits;

    fn deref(&self) -> &Self::Target {
        self.internal_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> DerefMut for InlAwi<BW, LEN> {
    fn deref_mut(&mut self) -> &mut Bits {
        self.internal_as_mut()
    }
}

impl<const BW: usize, const LEN: usize> Index<RangeFull> for InlAwi<BW, LEN> {
    type Output = Bits;

    fn index(&self, _i: RangeFull) -> &Bits {
        self
    }
}

impl<const BW: usize, const LEN: usize> Borrow<Bits> for InlAwi<BW, LEN> {
    fn borrow(&self) -> &Bits {
        self
    }
}

impl<const BW: usize, const LEN: usize> AsRef<Bits> for InlAwi<BW, LEN> {
    fn as_ref(&self) -> &Bits {
        self
    }
}

impl<const BW: usize, const LEN: usize> IndexMut<RangeFull> for InlAwi<BW, LEN> {
    fn index_mut(&mut self, _i: RangeFull) -> &mut Bits {
        self
    }
}

impl<const BW: usize, const LEN: usize> BorrowMut<Bits> for InlAwi<BW, LEN> {
    fn borrow_mut(&mut self) -> &mut Bits {
        self
    }
}

impl<const BW: usize, const LEN: usize> AsMut<Bits> for InlAwi<BW, LEN> {
    fn as_mut(&mut self) -> &mut Bits {
        self
    }
}

impl<const BW: usize, const LEN: usize> fmt::Debug for InlAwi<BW, LEN> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InlAwi({:?})", self.state())
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
        let mut val = Self::zero();
        val.const_as_mut().bool_(x);
        val
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
                    let mut val = Self::zero();
                    val.const_as_mut().$u_(x);
                    val
                }

                pub fn $from_i(x: impl Into<dag::$i>) -> Self {
                    let mut val = Self::zero();
                    val.const_as_mut().$i_(x);
                    val
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

type UsizeInlAwi = InlAwi<{ USIZE_BITS }, { awi::Bits::unstable_raw_digits(USIZE_BITS) }>;

impl UsizeInlAwi {
    pub fn from_usize(x: impl Into<dag::usize>) -> Self {
        let mut val = Self::zero();
        val.const_as_mut().usize_(x);
        val
    }

    pub fn from_isize(x: impl Into<dag::isize>) -> Self {
        let mut val = Self::zero();
        val.const_as_mut().isize_(x);
        val
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

/// Mimicking [awint_ext::ExtAwi]
///
/// Note: `extawi!(opaque: ..64)` just works
#[derive(Clone)]
#[repr(C)] // needed for `internal_as_ref*`, also this needs to just be a `PState`
pub struct ExtAwi {
    _no_send_or_sync: PhantomData<fn() -> Rc<()>>,
    pub(in crate::mimick) _state: PState,
}

impl Lineage for ExtAwi {
    fn state(&self) -> PState {
        self._state
    }
}

/// # Note
///
/// These functions are all mimicks of functions for [awint_ext::ExtAwi], except
/// for the special `arg`, `opaque`, and `opaque_with`.
impl ExtAwi {
    /// Special mimick-only function, most users should be using other
    /// construction methods
    pub fn from_state(state: PState) -> Self {
        Self {
            _no_send_or_sync: PhantomData,
            _state: state,
        }
    }

    /// Special mimick-only function, most users should be using other
    /// construction methods
    pub fn new(nzbw: NonZeroUsize, op: Op<PState>) -> Self {
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

    /// Constructs with the special `Op::Argument` state
    pub fn arg(arg: &awi::Bits) -> Self {
        Self::from_state(PState::new(
            arg.nzbw(),
            Op::Argument(awi::Awi::from_bits(arg)),
            None,
        ))
    }

    /// Constructs with the special `Op::Opaque` state, with a `None` name and
    /// no arguments
    pub fn opaque(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Opaque(smallvec![], None))
    }

    /// Constructs with the special `Op::Opaque` state, with custom bitwidth,
    /// name, and arguments
    pub fn opaque_with(w: NonZeroUsize, name: &'static str, with: &[&Bits]) -> Self {
        let mut v = smallvec![];
        for x in with {
            v.push(x.state());
        }
        Self::new(w, Op::Opaque(v, Some(name)))
    }

    pub fn zero(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::zero(w)))
    }

    pub fn umax(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::umax(w)))
    }

    pub fn imax(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::imax(w)))
    }

    pub fn imin(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::imin(w)))
    }

    pub fn uone(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::uone(w)))
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
        self.internal_as_ref()
    }
}

impl DerefMut for ExtAwi {
    fn deref_mut(&mut self) -> &mut Bits {
        self.internal_as_mut()
    }
}

impl Index<RangeFull> for ExtAwi {
    type Output = Bits;

    fn index(&self, _i: RangeFull) -> &Bits {
        self
    }
}

impl Borrow<Bits> for ExtAwi {
    fn borrow(&self) -> &Bits {
        self
    }
}

impl AsRef<Bits> for ExtAwi {
    fn as_ref(&self) -> &Bits {
        self
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
        write!(f, "ExtAwi({:?})", self.state())
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
        Self::new(bits.nzbw(), Op::Literal(awi::Awi::from(bits)))
    }
}

// there are some bizaare trait conflicts if we don't enumerate all these cases

impl From<&awi::ExtAwi> for ExtAwi {
    fn from(bits: &awi::ExtAwi) -> ExtAwi {
        Self::new(bits.nzbw(), Op::Literal(awi::Awi::from(bits.as_ref())))
    }
}

impl From<awi::ExtAwi> for ExtAwi {
    fn from(bits: awi::ExtAwi) -> ExtAwi {
        Self::new(bits.nzbw(), Op::Literal(awi::Awi::from(bits.as_ref())))
    }
}

impl<const BW: usize, const LEN: usize> From<InlAwi<BW, LEN>> for ExtAwi {
    fn from(awi: InlAwi<BW, LEN>) -> ExtAwi {
        Self::from_state(awi.state())
    }
}

impl<const BW: usize, const LEN: usize> From<&InlAwi<BW, LEN>> for ExtAwi {
    fn from(awi: &InlAwi<BW, LEN>) -> ExtAwi {
        Self::from_state(awi.state())
    }
}

impl<const BW: usize, const LEN: usize> From<awi::InlAwi<BW, LEN>> for ExtAwi {
    fn from(awi: awi::InlAwi<BW, LEN>) -> ExtAwi {
        Self::new(awi.nzbw(), Op::Literal(awi::Awi::from(awi.as_ref())))
    }
}

impl<const BW: usize, const LEN: usize> From<&awi::InlAwi<BW, LEN>> for ExtAwi {
    fn from(awi: &awi::InlAwi<BW, LEN>) -> ExtAwi {
        Self::new(awi.nzbw(), Op::Literal(awi::Awi::from(awi.as_ref())))
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

/// Mimicking [awint_ext::Awi]
///
/// Note: `awi!(opaque: ..64)` just works
#[derive(Clone)]
#[repr(C)] // needed for `internal_as_ref*`, also this needs to just be a `PState`
pub struct Awi {
    _no_send_or_sync: PhantomData<fn() -> Rc<()>>,
    pub(in crate::mimick) _state: PState,
}

impl Lineage for Awi {
    fn state(&self) -> PState {
        self._state
    }
}

/// # Note
///
/// These functions are all mimicks of functions for [awint_ext::Awi], except
/// for the special `arg`, `opaque`, and `opaque_with`.
///
/// `Awi::shrink_to_msb` does not have a mimicking version, because it modifies
/// the bitwidth based on a value that could be dynamic
impl Awi {
    /// Special mimick-only function, most users should be using other
    /// construction methods
    pub fn from_state(state: PState) -> Self {
        Self {
            _no_send_or_sync: PhantomData,
            _state: state,
        }
    }

    /// Special mimick-only function, most users should be using other
    /// construction methods
    pub fn new(nzbw: NonZeroUsize, op: Op<PState>) -> Self {
        Self::from_state(PState::new(nzbw, op, None))
    }

    pub fn nzbw(&self) -> NonZeroUsize {
        self.state_nzbw()
    }

    pub fn bw(&self) -> usize {
        self.nzbw().get()
    }

    pub fn from_bits(bits: &Bits) -> Self {
        Self::from_state(bits.state())
    }

    /// Constructs with the special `Op::Argument` state
    pub fn arg(arg: &awi::Bits) -> Self {
        Self::from_state(PState::new(
            arg.nzbw(),
            Op::Argument(awi::Awi::from_bits(arg)),
            None,
        ))
    }

    /// Constructs with the special `Op::Opaque` state, with a `None` name and
    /// no arguments
    pub fn opaque(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Opaque(smallvec![], None))
    }

    /// Constructs with the special `Op::Opaque` state, with custom bitwidth,
    /// name, and arguments
    pub fn opaque_with(w: NonZeroUsize, name: &'static str, with: &[&Bits]) -> Self {
        let mut v = smallvec![];
        for x in with {
            v.push(x.state());
        }
        Self::new(w, Op::Opaque(v, Some(name)))
    }

    pub fn zero(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::zero(w)))
    }

    pub fn umax(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::umax(w)))
    }

    pub fn imax(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::imax(w)))
    }

    pub fn imin(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::imin(w)))
    }

    pub fn uone(w: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::uone(w)))
    }

    pub fn from_bits_with_capacity(bits: &Bits, _min_capacity: NonZeroUsize) -> Awi {
        Self::from_state(bits.state())
    }

    pub fn zero_with_capacity(w: NonZeroUsize, _min_capacity: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::zero(w)))
    }

    pub fn umax_with_capacity(w: NonZeroUsize, _min_capacity: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::umax(w)))
    }

    pub fn imax_with_capacity(w: NonZeroUsize, _min_capacity: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::imax(w)))
    }

    pub fn imin_with_capacity(w: NonZeroUsize, _min_capacity: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::imin(w)))
    }

    pub fn uone_with_capacity(w: NonZeroUsize, _min_capacity: NonZeroUsize) -> Self {
        Self::new(w, Op::Literal(awi::Awi::uone(w)))
    }

    pub fn reserve(&mut self, _additional: usize) {
        let _ = self;
    }

    pub fn shrink_to(&mut self, _min_capacity: usize) {
        let _ = self;
    }

    pub fn shrink_to_fit(&mut self) {
        let _ = self;
    }

    pub fn resize(&mut self, new_bitwidth: NonZeroUsize, extension: impl Into<dag::bool>) {
        let tmp = self.clone();
        *self = Self::zero(new_bitwidth);
        self.resize_(&tmp, extension)
    }

    pub fn zero_resize(&mut self, new_bitwidth: NonZeroUsize) {
        let tmp = self.clone();
        *self = Self::zero(new_bitwidth);
        self.zero_resize_(&tmp);
    }

    pub fn sign_resize(&mut self, new_bitwidth: NonZeroUsize) {
        let tmp = self.clone();
        *self = Self::zero(new_bitwidth);
        self.zero_resize_(&tmp);
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

impl Deref for Awi {
    type Target = Bits;

    fn deref(&self) -> &Self::Target {
        self.internal_as_ref()
    }
}

impl DerefMut for Awi {
    fn deref_mut(&mut self) -> &mut Bits {
        self.internal_as_mut()
    }
}

impl Index<RangeFull> for Awi {
    type Output = Bits;

    fn index(&self, _i: RangeFull) -> &Bits {
        self
    }
}

impl Borrow<Bits> for Awi {
    fn borrow(&self) -> &Bits {
        self
    }
}

impl AsRef<Bits> for Awi {
    fn as_ref(&self) -> &Bits {
        self
    }
}

impl IndexMut<RangeFull> for Awi {
    fn index_mut(&mut self, _i: RangeFull) -> &mut Bits {
        self.const_as_mut()
    }
}

impl BorrowMut<Bits> for Awi {
    fn borrow_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

impl AsMut<Bits> for Awi {
    fn as_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

impl fmt::Debug for Awi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Awi({:?})", self.state())
    }
}

forward_debug_fmt!(Awi);

impl From<&Bits> for Awi {
    fn from(bits: &Bits) -> Awi {
        Self::from_state(bits.state())
    }
}

impl From<&awi::Bits> for Awi {
    fn from(bits: &awi::Bits) -> Awi {
        Self::new(bits.nzbw(), Op::Literal(awi::Awi::from(bits)))
    }
}

// there are some bizaare trait conflicts if we don't enumerate all these cases

impl From<&awi::Awi> for Awi {
    fn from(bits: &awi::Awi) -> Awi {
        Self::new(bits.nzbw(), Op::Literal(awi::Awi::from(bits.as_ref())))
    }
}

impl From<awi::Awi> for Awi {
    fn from(bits: awi::Awi) -> Awi {
        Self::new(bits.nzbw(), Op::Literal(awi::Awi::from(bits.as_ref())))
    }
}

impl<const BW: usize, const LEN: usize> From<InlAwi<BW, LEN>> for Awi {
    fn from(awi: InlAwi<BW, LEN>) -> Awi {
        Self::from_state(awi.state())
    }
}

impl<const BW: usize, const LEN: usize> From<&InlAwi<BW, LEN>> for Awi {
    fn from(awi: &InlAwi<BW, LEN>) -> Awi {
        Self::from_state(awi.state())
    }
}

impl<const BW: usize, const LEN: usize> From<awi::InlAwi<BW, LEN>> for Awi {
    fn from(awi: awi::InlAwi<BW, LEN>) -> Awi {
        Self::new(awi.nzbw(), Op::Literal(awi::Awi::from(awi.as_ref())))
    }
}

impl<const BW: usize, const LEN: usize> From<&awi::InlAwi<BW, LEN>> for Awi {
    fn from(awi: &awi::InlAwi<BW, LEN>) -> Awi {
        Self::new(awi.nzbw(), Op::Literal(awi::Awi::from(awi.as_ref())))
    }
}

macro_rules! extawi_from {
    ($($ty:ident, $from:ident);*;) => {
        $(
            impl Awi {
                pub fn $from(x: impl Into<dag::$ty>) -> Self {
                    Self::from(InlAwi::$from(x))
                }
            }

            impl From<dag::$ty> for Awi {
                fn from(x: dag::$ty) -> Self {
                    Self::$from(x)
                }
            }

            impl From<awi::$ty> for Awi {
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

// misc

impl<const BW: usize, const LEN: usize> From<awi::InlAwi<BW, LEN>> for InlAwi<BW, LEN> {
    fn from(val: awi::InlAwi<BW, LEN>) -> InlAwi<BW, LEN> {
        let val = Awi::from(val);
        Self::from_state(val.state())
    }
}

impl<const BW: usize, const LEN: usize> From<&awi::InlAwi<BW, LEN>> for InlAwi<BW, LEN> {
    fn from(val: &awi::InlAwi<BW, LEN>) -> InlAwi<BW, LEN> {
        let val = Awi::from(val);
        Self::from_state(val.state())
    }
}
