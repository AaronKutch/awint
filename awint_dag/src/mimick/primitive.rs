use std::{fmt, num::NonZeroUsize, ops::*};

use awint_ext::{awi, awint_internals::*, Awi};
use awint_macro_internals::triple_arena::Ptr;

use crate::{dag, mimick::InlAwi, Lineage, Op, PState};

macro_rules! unary {
    ($name:ident; $($std_trait:ident $std_fn:ident $assign_name:ident),*,) => {
        $(
            impl $std_trait for $name {
                type Output = Self;

                fn $std_fn(self) -> Self {
                    let mut tmp = self.clone();
                    tmp.0.$assign_name();
                    tmp
                }
            }
        )*
    };
}

macro_rules! op_ {
    ($name:ident; $($std_trait:ident $std_fn:ident $assign_name:ident),*,) => {
        $(
            impl<I> $std_trait<I> for $name where I: Into<dag::$name> {
                fn $std_fn(&mut self, rhs: I) where I: Into<dag::$name> {
                    self.0.$assign_name(&rhs.into().0).unwrap();
                }
            }
        )*
    };
}

macro_rules! triop {
    ($name:ident; $($std_trait:ident $std_fn:ident $op_:ident),*,) => {
        $(
            impl $std_trait for $name {
                type Output = Self;

                fn $std_fn(self, rhs: Self) -> Self {
                    let mut tmp = self.clone();
                    tmp.0.$op_(&rhs.0).unwrap();
                    tmp
                }
            }

            impl $std_trait<awi::$name> for $name {
                type Output = Self;

                fn $std_fn(self, rhs: awi::$name) -> Self {
                    let mut tmp = self.clone();
                    tmp.0.$op_(&$name::from(rhs).0).unwrap();
                    tmp
                }
            }

            impl $std_trait<dag::$name> for awi::$name {
                type Output = dag::$name;

                fn $std_fn(self, rhs: dag::$name) -> Self::Output {
                    let mut tmp = rhs.clone();
                    tmp.0.$op_(&$name::from(self).0).unwrap();
                    tmp
                }
            }
        )*
    };
}

macro_rules! prim {
    ($($name:ident $assign:ident $w:expr),*,) => {
        $(
            /// Mimicking primitive of same name
            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy)]
            pub struct $name(InlAwi<$w, {awi::Bits::unstable_raw_digits($w)}>);

            impl Lineage for $name {
                fn state(&self) -> PState {
                    self.0.state()
                }
            }

            impl $name {
                pub(crate) fn from_state(state: PState) -> Self {
                    Self(InlAwi::from_state(state))
                }

                pub(crate) fn new_lit(lit: Awi) -> Self {
                    core::debug_assert_eq!(lit.bw(), $w);
                    Self::from_state(PState::new(
                        NonZeroUsize::new($w).unwrap(),
                        Op::Literal(lit),
                        None
                    ))
                }

                pub(crate) fn new_eager_eval(op: Op<PState>) -> crate::mimick::Option<Self> {
                    let mut r = Self::from_state(PState::invalid());
                    match r.0.update_state(bw($w), op) {
                        dag::Option::None => dag::Option::None,
                        dag::Option::Some(()) => dag::Option::Some(r),
                        dag::Option::Opaque(_) => unreachable!(),
                    }
                }

                pub(crate) fn get_nzbw() -> NonZeroUsize {
                    NonZeroUsize::new($w).unwrap()
                }

                pub fn wrapping_add(mut self, rhs: impl Into<Self>) -> Self {
                    let _ = self.0.add_(&rhs.into().0);
                    self
                }

                pub fn wrapping_sub(mut self, rhs: impl Into<Self>) -> Self {
                    let _ = self.0.sub_(&rhs.into().0);
                    self
                }
            }

            impl From<awi::$name> for $name {
                fn from(x: awi::$name) -> Self {
                    Self::new_lit(awi::Awi::from(x))
                }
            }

            impl fmt::Debug for $name {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    write!(f, "{}({:?})", stringify!($name), self.0.state())
                }
            }

            forward_debug_fmt!($name);

            unary!($name;
                Not not not_,
            );

            op_!($name;
                AddAssign add_assign add_,
                SubAssign sub_assign sub_,
                BitOrAssign bitor_assign or_,
                BitAndAssign bitand_assign and_,
                BitXorAssign bitxor_assign xor_,
            );

            triop!($name;
                Add add add_,
                Sub sub sub_,
                BitOr bitor or_,
                BitAnd bitand and_,
                BitXor bitxor xor_,
            );
        )*
    };
}

/// Mimicking primitive of same name
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct bool(InlAwi<1, { awi::Bits::unstable_raw_digits(1) }>);

impl Lineage for bool {
    fn state(&self) -> PState {
        self.0.state()
    }
}

impl bool {
    pub(crate) fn from_state(state: PState) -> Self {
        Self(InlAwi::from_state(state))
    }

    pub(crate) fn new_lit(lit: awi::bool) -> Self {
        Self::from_state(PState::new(
            NonZeroUsize::new(1).unwrap(),
            Op::Literal(Awi::from_bool(lit)),
            None,
        ))
    }

    pub(crate) fn new_eager_eval(op: Op<PState>) -> crate::mimick::Option<Self> {
        let mut r = Self::from_state(PState::invalid());
        match r.0.update_state(bw(1), op) {
            dag::Option::None => dag::Option::None,
            dag::Option::Some(()) => dag::Option::Some(r),
            dag::Option::Opaque(_) => unreachable!(),
        }
    }

    pub(crate) fn get_nzbw() -> NonZeroUsize {
        NonZeroUsize::new(1).unwrap()
    }
}

impl From<awi::bool> for bool {
    fn from(x: awi::bool) -> Self {
        Self::new_lit(x)
    }
}

impl fmt::Debug for bool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bool({:?})", self.0.state())
    }
}

forward_debug_fmt!(bool);

unary!(bool;
    Not not not_,
);

op_!(bool;
    BitOrAssign bitor_assign or_,
    BitAndAssign bitand_assign and_,
    BitXorAssign bitxor_assign xor_,
);

triop!(bool;
    BitOr bitor or_,
    BitAnd bitand and_,
    BitXor bitxor xor_,
);

prim!(
    u8 U8Assign 8,
    u16 U16Assign 16,
    u32 U32Assign 32,
    u64 U64Assign 64,
    u128 U128Assign 128,
    usize UsizeAssign USIZE_BITS,
    i8 I8Assign 8,
    i16 I16Assign 16,
    i32 I32Assign 32,
    i64 I64Assign 64,
    i128 I128Assign 128,
    isize IsizeAssign USIZE_BITS,
);
