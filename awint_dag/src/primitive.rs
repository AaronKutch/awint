use std::num::NonZeroUsize;

use awint_internals::{bw, BITS};
use triple_arena::{Arena, TriPtr};
use std::ops::*;

use crate::{Lineage, Op};
use Op::InitCopy;

macro_rules! prim {
    ($($name:ident $bw:expr),*,) => {
        $(
            #[allow(non_camel_case_types)]
            #[derive(Debug)]
            pub struct $name {
                state: TriPtr,
                ops: Arena<Op>,
            }

            impl $name {
                pub fn new(init_op: Op) -> Self {
                    let mut a = Arena::new();
                    Self {
                        state: a.insert(init_op),
                        ops: a,
                    }
                }
            }

            impl Lineage for $name {
                fn state(&self) -> TriPtr {
                    self.state
                }

                fn ops(&self) -> &Arena<Op> {
                    &self.ops
                }

                fn nzbw(&self) -> NonZeroUsize {
                    bw($bw)
                }
            }
        )*
    };
}

prim!(
    bool 1,
    usize BITS,
    isize BITS,
    u8 8,
    i8 8,
    u16 16,
    i16 16,
    u32 32,
    i32 32,
    u64 64,
    i64 64,
    u128 128,
    i128 128,
);

macro_rules! impl_integral_traits {
    ($($name:ident $dagprim:ident),*,) => {
        $(
            /*impl<I> AddAssign<I> for $name {
                fn add_assign(&mut self, rhs: I) where I: Into<$dagprim> {
                    self.state = self.ops.insert(Op::AddAssign(self.state, rhs.???));
                }
            }*/

            impl SubAssign for $name {
                fn sub_assign(&mut self, rhs: Self) {
                    self.state = self.ops.insert(Op::SubAssign(self.state, rhs.state));
                }
            }

            impl Add for $name {
                type Output = Self;

                fn add(self, rhs: Self) -> Self {
                    let mut tmp = Self::new(InitCopy(self.state));
                    tmp.state = tmp.ops.insert(Op::AddAssign(self.state, rhs.state));
                    tmp
                }
            }

            impl Sub for $name {
                type Output = Self;

                fn sub(self, rhs: Self) -> Self {
                    let mut tmp = Self::new(InitCopy(self.state));
                    tmp.state = tmp.ops.insert(Op::SubAssign(self.state, rhs.state));
                    tmp
                }
            }

        )*
    };
}

impl_integral_traits!(
    usize DagUsize,
    isize DagIsize,
    u8 DagU8,
    i8 DagI8,
    u16 DagU16,
    i16 DagI16,
    u32 DagU32,
    i32 DagI32,
    u64 DagU64,
    i64 DagI64,
    u128 DagU128,
    i128 DagI128,
);
