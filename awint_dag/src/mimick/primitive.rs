use std::{fmt, num::NonZeroUsize, ops::*};

use awint_internals::*;

use crate::{
    common::{Lineage, Op, PState},
    mimick::{primitive as prim, InlAwi},
};

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

macro_rules! op_assign {
    ($name:ident; $($std_trait:ident $std_fn:ident $assign_name:ident),*,) => {
        $(
            impl<I> $std_trait<I> for $name where I: Into<prim::$name> {
                fn $std_fn(&mut self, rhs: I) where I: Into<prim::$name> {
                    self.0.$assign_name(&rhs.into().0).unwrap();
                }
            }
        )*
    };
}

macro_rules! triop {
    ($name:ident; $($std_trait:ident $std_fn:ident $op_assign:ident),*,) => {
        $(
            impl $std_trait for $name {
                type Output = Self;

                fn $std_fn(self, rhs: Self) -> Self {
                    let mut tmp = self.clone();
                    tmp.0.$op_assign(&rhs.0).unwrap();
                    tmp
                }
            }

            impl $std_trait<core::primitive::$name> for $name {
                type Output = Self;

                fn $std_fn(self, rhs: core::primitive::$name) -> Self {
                    let mut tmp = self.clone();
                    tmp.0.$op_assign(&$name::from(rhs).0).unwrap();
                    tmp
                }
            }
        )*
    };
}

macro_rules! prim {
    ($($name:ident $assign:ident $bw:expr),*,) => {
        $(
            /// Mimicking primitive of same name
            #[allow(non_camel_case_types)]
            pub struct $name(InlAwi<$bw, {crate::mimick::Bits::unstable_raw_digits($bw)}>);

            impl Lineage for $name {
                fn hidden_const_nzbw() -> Option<NonZeroUsize> {
                    Some(bw($bw))
                }

                fn state(&self) -> PState {
                    self.0.state()
                }
            }

            impl $name {
                pub(crate) fn new(op: Op<PState>) -> Self {
                    Self(InlAwi::new(op))
                }
            }

            impl From<core::primitive::$name> for $name {
                fn from(x: core::primitive::$name) -> Self {
                    Self::new(Op::Literal(awint_ext::ExtAwi::from(x)))
                }
            }

            impl Clone for $name {
                fn clone(&self) -> Self {
                    Self::new(Op::Copy([self.state()]))
                }
            }

            impl fmt::Debug for $name {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    write!(f, "{}({:?})", stringify!($name), self.0.state())
                }
            }

            forward_debug_fmt!($name);

            unary!($name;
                Not not not_assign,
            );

            op_assign!($name;
                AddAssign add_assign add_assign,
                SubAssign sub_assign sub_assign,
                BitOrAssign bitor_assign or_assign,
                BitAndAssign bitand_assign and_assign,
                BitXorAssign bitxor_assign xor_assign,
            );

            triop!($name;
                Add add add_assign,
                Sub sub sub_assign,
                BitOr bitor or_assign,
                BitAnd bitand and_assign,
                BitXor bitxor xor_assign,
            );
        )*
    };
}

/// Mimicking primitive of same name
#[allow(non_camel_case_types)]
pub struct bool(InlAwi<1, { crate::mimick::Bits::unstable_raw_digits(1) }>);

impl Lineage for bool {
    fn hidden_const_nzbw() -> Option<NonZeroUsize> {
        Some(bw(1))
    }

    fn state(&self) -> PState {
        self.0.state()
    }
}

impl bool {
    pub(crate) fn new(op: Op<PState>) -> Self {
        Self(InlAwi::new(op))
    }
}

impl From<core::primitive::bool> for bool {
    fn from(x: core::primitive::bool) -> Self {
        Self::new(Op::Literal(awint_ext::ExtAwi::from(x)))
    }
}

impl Clone for bool {
    fn clone(&self) -> Self {
        Self::new(Op::Copy([self.state()]))
    }
}

impl fmt::Debug for bool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bool({:?})", self.0.state())
    }
}

forward_debug_fmt!(bool);

unary!(bool;
    Not not not_assign,
);

op_assign!(bool;
    BitOrAssign bitor_assign or_assign,
    BitAndAssign bitand_assign and_assign,
    BitXorAssign bitxor_assign xor_assign,
);

triop!(bool;
    BitOr bitor or_assign,
    BitAnd bitand and_assign,
    BitXor bitxor xor_assign,
);

prim!(
    u8 U8Assign 8,
    u16 U16Assign 16,
    u32 U32Assign 32,
    u64 U64Assign 64,
    u128 U128Assign 128,
    usize UsizeAssign BITS,
    i8 I8Assign 8,
    i16 I16Assign 16,
    i32 I32Assign 32,
    i64 I64Assign 64,
    i128 I128Assign 128,
    isize IsizeAssign BITS,
);
