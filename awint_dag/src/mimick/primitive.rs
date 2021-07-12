use std::{num::NonZeroUsize, ops::*, rc::Rc};

use awint_internals::{bw, BITS};

use crate::mimick::{primitive as prim, Bits, Lineage, Op};

macro_rules! prim {
    ($($name:ident $assign:ident $bw:expr),*,) => {
        $(
            /// Mimicking primitive of same name
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone)]
            pub struct $name(Bits);

            impl $name {
                pub(crate) fn new(op: Op) -> Self {
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
                    Self(Bits::new(bw($bw), Op::$assign(x)))
                }
            }

            impl<I> AddAssign<I> for $name where I: Into<prim::$name> {
                fn add_assign(&mut self, rhs: I) where I: Into<prim::$name> {
                    self.update(Op::AddAssign(self.op(), rhs.into().op()));
                }
            }

            impl<I> SubAssign<I> for $name where I: Into<prim::$name> {
                fn sub_assign(&mut self, rhs: I) where I: Into<prim::$name> {
                    self.update(Op::SubAssign(self.op(), rhs.into().op()));
                }
            }

            impl Add for $name {
                type Output = Self;

                fn add(self, rhs: Self) -> Self {
                    let mut tmp = self.clone();
                    tmp.update(Op::AddAssign(tmp.op(), rhs.op()));
                    tmp
                }
            }
        )*
    };
}

prim!(
    bool LitBoolAssign 1,
    usize LitUsizeAssign BITS,
    isize LitIsizeAssign BITS,
    u8 LitU8Assign 8,
    i8 LitI8Assign 8,
    u16 LitU16Assign 16,
    i16 LitI16Assign 16,
    u32 LitU32Assign 32,
    i32 LitI32Assign 32,
    u64 LitU64Assign 64,
    i64 LitI64Assign 64,
    u128 LitU128Assign 128,
    i128 LitI128Assign 128,
);
