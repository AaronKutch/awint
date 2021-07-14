use std::{num::NonZeroUsize, ops::*, rc::Rc};

use awint_internals::{bw, BITS};

use crate::mimick::{primitive as prim, Bits, Lineage, Op};

macro_rules! op_assign {
    ($name:ident; $($std_trait:ident $std_fn:ident $op:ident),*,) => {
        $(
            impl<I> $std_trait<I> for $name where I: Into<prim::$name> {
                fn $std_fn(&mut self, rhs: I) where I: Into<prim::$name> {
                    self.update(Op::$op(self.op(), rhs.into().op()));
                }
            }
        )*
    };
}

macro_rules! triop {
    ($name:ident; $($std_trait:ident $std_fn:ident $op:ident),*,) => {
        $(
            impl $std_trait for $name {
                type Output = Self;

                fn $std_fn(self, rhs: Self) -> Self {
                    let mut tmp = self.clone();
                    tmp.update(Op::$op(tmp.op(), rhs.op()));
                    tmp
                }
            }

            impl $std_trait<core::primitive::$name> for $name {
                type Output = Self;

                fn $std_fn(self, rhs: core::primitive::$name) -> Self {
                    let mut tmp = self.clone();
                    let rhs = Self::from(rhs);
                    tmp.update(Op::$op(tmp.op(), rhs.op()));
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
            #[derive(Debug, Hash, PartialEq, Eq)]
            pub struct $name(Bits);

            impl $name {
                pub(crate) fn from_op(op: Op) -> Self {
                    Self(Bits::new(bw($bw), op))
                }
            }

            impl Lineage for $name {
                fn nzbw(&self) -> NonZeroUsize {
                    self.0.nzbw()
                }

                fn op(&self) -> Rc<Op> {
                    self.0.op()
                }

                fn op_mut(&mut self) -> &mut Rc<Op> {
                    self.0.op_mut()
                }
            }

            impl From<core::primitive::$name> for $name {
                fn from(x: core::primitive::$name) -> Self {
                    Self::from_op(Op::LitAssign(awint_ext::ExtAwi::from(x)))
                }
            }

            impl Clone for $name {
                fn clone(&self) -> Self {
                    Self::from_op(Op::CopyAssign(self.op()))
                }
            }

            op_assign!($name;
                AddAssign add_assign AddAssign,
                SubAssign sub_assign SubAssign,
                BitOrAssign bitor_assign OrAssign,
                BitAndAssign bitand_assign AndAssign,
                BitXorAssign bitxor_assign XorAssign,
            );

            triop!($name;
                Add add AddAssign,
                Sub sub SubAssign,
                BitOr bitor OrAssign,
                BitAnd bitand AndAssign,
                BitXor bitxor XorAssign,
            );
        )*
    };
}

prim!(
    bool BoolAssign 1,
    usize UsizeAssign BITS,
    isize IsizeAssign BITS,
    u8 U8Assign 8,
    i8 I8Assign 8,
    u16 U16Assign 16,
    i16 I16Assign 16,
    u32 U32Assign 32,
    i32 I32Assign 32,
    u64 U64Assign 64,
    i64 I64Assign 64,
    u128 U128Assign 128,
    i128 I128Assign 128,
);
