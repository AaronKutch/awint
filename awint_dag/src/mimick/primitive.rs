use std::{num::NonZeroUsize, ops::*, rc::Rc};

use awint_internals::BITS;

use crate::{
    mimick::{primitive as prim, Bits, ConstBwLineage, Lineage, State},
    Op,
};

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
    ($name:ident; $($std_trait:ident $std_fn:ident $op:ident),*,) => {
        $(
            impl $std_trait for $name {
                type Output = Self;

                fn $std_fn(self, rhs: Self) -> Self {
                    let mut tmp = self.clone();
                    tmp.0.add_assign(&rhs.0).unwrap();
                    tmp
                }
            }

            impl $std_trait<core::primitive::$name> for $name {
                type Output = Self;

                fn $std_fn(self, rhs: core::primitive::$name) -> Self {
                    let mut tmp = self.clone();
                    tmp.0.add_assign(&$name::from(rhs).0).unwrap();
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

            impl ConstBwLineage for $name {
                fn new(op: Op, ops: Vec<Rc<State>>) -> Self {
                    Self(Bits::new(Self::hidden_const_nzbw(), op, ops))
                }

                fn hidden_const_nzbw() -> NonZeroUsize {
                    NonZeroUsize::new($bw).unwrap()
                }

                fn state(&self) -> Rc<State> {
                    self.0.state()
                }
            }

            impl From<core::primitive::$name> for $name {
                fn from(x: core::primitive::$name) -> Self {
                    Self::new(Op::Literal(awint_ext::ExtAwi::from(x)), vec![])
                }
            }

            impl Clone for $name {
                fn clone(&self) -> Self {
                    Self::new(Op::CopyAssign, vec![self.state()])
                }
            }

            op_assign!($name;
                AddAssign add_assign add_assign,
                SubAssign sub_assign sub_assign,
                BitOrAssign bitor_assign or_assign,
                BitAndAssign bitand_assign and_assign,
                BitXorAssign bitxor_assign xor_assign,
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
