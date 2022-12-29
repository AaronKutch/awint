use core::option::Option as StdOption;
use std::{
    borrow::{Borrow, BorrowMut},
    mem,
    ops::{Deref, DerefMut},
};

use awint_ext::awi;
use StdOption::{None as StdNone, Some as StdSome};

use crate::{common::register_assertion_bit, dag, mimick::*};

// the type itself must be public, but nothing else about it can
#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct OpaqueInternal<T> {
    pub(in crate::mimick) is_some: dag::bool,
    pub(in crate::mimick) t: StdOption<T>,
}

impl<T> OpaqueInternal<T> {
    fn as_ref(&self) -> OpaqueInternal<&T> {
        OpaqueInternal {
            is_some: self.is_some,
            t: self.t.as_ref(),
        }
    }

    fn as_mut(&mut self) -> OpaqueInternal<&mut T> {
        OpaqueInternal {
            is_some: self.is_some,
            t: self.t.as_mut(),
        }
    }

    fn copied(self) -> OpaqueInternal<T>
    where
        T: Clone,
    {
        OpaqueInternal {
            is_some: self.is_some,
            t: self.t,
        }
    }

    fn cloned(self) -> OpaqueInternal<T>
    where
        T: Clone,
    {
        OpaqueInternal {
            is_some: self.is_some,
            t: self.t,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Option<T> {
    None,
    Some(T),
    Opaque(OpaqueInternal<T>),
}

use crate::mimick::Option::Opaque;
pub use crate::mimick::Option::{None, Some};

impl<T> From<awi::Option<T>> for dag::Option<T> {
    fn from(value: awi::Option<T>) -> Self {
        match value {
            awi::None => None,
            awi::Some(t) => Some(t),
        }
    }
}

impl<T> Option<T> {
    pub fn as_ref(&self) -> Option<&T> {
        match *self {
            None => None,
            Some(ref t) => Some(t),
            Opaque(ref z) => Opaque(z.as_ref()),
        }
    }

    pub fn as_mut(&mut self) -> Option<&mut T> {
        match *self {
            None => None,
            Some(ref mut t) => Some(t),
            Opaque(ref mut z) => Opaque(z.as_mut()),
        }
    }

    pub fn as_deref(&self) -> Option<&<T as Deref>::Target>
    where
        T: Deref,
    {
        match self.as_ref() {
            None => None,
            Some(t) => Some(t),
            // need to write this out for some reason
            Opaque(z) => Opaque(OpaqueInternal {
                is_some: z.is_some,
                t: match z.t {
                    StdNone => StdNone,
                    StdSome(t) => StdSome(Deref::deref(t)),
                },
            }),
        }
    }

    pub fn as_deref_mut(&mut self) -> Option<&mut <T as Deref>::Target>
    where
        T: DerefMut,
    {
        match self.as_mut() {
            None => None,
            Some(t) => Some(t),
            // need to write this out for some reason
            Opaque(z) => Opaque(OpaqueInternal {
                is_some: z.is_some,
                t: match z.t {
                    StdNone => StdNone,
                    StdSome(t) => StdSome(DerefMut::deref_mut(t)),
                },
            }),
        }
    }

    pub fn copied(self) -> Option<T>
    where
        T: Copy,
    {
        match self {
            None => None,
            Some(t) => Some(t),
            Opaque(z) => Opaque(z.copied()),
        }
    }

    pub fn cloned(self) -> Option<T>
    where
        T: Clone,
    {
        match self {
            None => None,
            Some(t) => Some(t),
            Opaque(z) => Opaque(z.cloned()),
        }
    }

    pub fn is_none_at_runtime(&self) -> bool {
        match self {
            None => true,
            Some(_) => false,
            Opaque(_) => false,
        }
    }

    pub fn is_none(&self) -> dag::bool {
        match self {
            None => true.into(),
            Some(_) => false.into(),
            Opaque(z) => !z.is_some,
        }
    }

    pub fn is_some_at_runtime(&self) -> bool {
        match self {
            None => false,
            Some(_) => true,
            Opaque(_) => false,
        }
    }

    pub fn is_some(&self) -> dag::bool {
        match self {
            None => false.into(),
            Some(_) => true.into(),
            Opaque(z) => z.is_some,
        }
    }

    pub fn is_opaque_at_runtime(&self) -> bool {
        match self {
            None => false,
            Some(_) => false,
            Opaque(_) => true,
        }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Option<U> {
        match self {
            None => None,
            Some(t) => Some(f(t)),
            Opaque(z) => Opaque(OpaqueInternal {
                is_some: z.is_some,
                t: z.t.map(f),
            }),
        }
    }

    #[track_caller]
    pub fn replace(&mut self, value: T) -> Option<T> {
        let res = mem::replace(self, None);
        *self = Some(value);
        res
    }

    pub fn take(&mut self) -> Option<T> {
        mem::replace(self, None)
    }

    #[track_caller]
    pub fn unwrap(self) -> T {
        match self {
            None => panic!("called `Option::unwrap()` on a `None` value"),
            Some(t) => t,
            Opaque(z) => {
                register_assertion_bit(z.is_some);
                if let StdSome(t) = z.t {
                    t
                } else {
                    panic!("called `Option::unwrap()` on an unrealizable `Opaque` value")
                }
            }
        }
    }
}

impl<T: Borrow<Bits> + BorrowMut<Bits>> Option<T> {
    #[track_caller]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            None => default,
            Some(t) => t,
            Opaque(z) => {
                if let StdSome(mut t) = z.t {
                    if t.borrow_mut()
                        .mux_(default.borrow(), z.is_some)
                        .is_none_at_runtime()
                    {
                        panic!("called `Option::unwrap_or()` with unequal bitwidth types")
                    }
                    t
                } else {
                    panic!("called `Option::unwrap_or()` on an unrealizable `Opaque` value")
                }
            }
        }
    }
}

#[cfg(feature = "try_support")]
impl<T> std::ops::Residual<T> for Option<!> {
    type TryType = Option<T>;
}

#[cfg(feature = "try_support")]
impl<T> std::ops::FromResidual<Option<!>> for Option<T> {
    fn from_residual(residual: Option<!>) -> Self {
        match residual {
            None => None,
            _ => unreachable!(),
        }
    }
}

#[cfg(feature = "try_support")]
impl<T> std::ops::Try for Option<T> {
    type Output = T;
    type Residual = Option<!>;

    fn from_output(output: Self::Output) -> Self {
        Some(output)
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        use std::ops::ControlFlow;
        match self {
            None => ControlFlow::Break(None),
            Some(t) => ControlFlow::Continue(t),
            Opaque(z) => {
                if let StdSome(t) = z.t {
                    ControlFlow::Continue(t)
                } else {
                    panic!("called `Try::branch` on an unrealizable `Opaque` value")
                }
            }
        }
    }
}
