use core::result::Result as StdResult;
use std::{
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    ops::{Deref, DerefMut},
    process::{ExitCode, Termination},
};

use awint_ext::{awi, awint_internals::Location};
use StdResult::{Err as StdErr, Ok as StdOk};

use crate::{dag, epoch::register_assertion_bit_for_current_epoch, mimick::*};

// the type itself must be public, but nothing else about it can
#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct OpaqueInternal<T, E> {
    pub(in crate::mimick) is_ok: dag::bool,
    pub(in crate::mimick) res: StdResult<T, E>,
}

impl<T, E> OpaqueInternal<T, E> {
    fn as_ref(&self) -> OpaqueInternal<&T, &E> {
        OpaqueInternal {
            is_ok: self.is_ok,
            res: self.res.as_ref(),
        }
    }

    fn as_mut(&mut self) -> OpaqueInternal<&mut T, &mut E> {
        OpaqueInternal {
            is_ok: self.is_ok,
            res: self.res.as_mut(),
        }
    }

    fn copied(self) -> OpaqueInternal<T, E>
    where
        T: Clone,
    {
        OpaqueInternal {
            is_ok: self.is_ok,
            res: self.res,
        }
    }

    fn cloned(self) -> OpaqueInternal<T, E>
    where
        T: Clone,
    {
        OpaqueInternal {
            is_ok: self.is_ok,
            res: self.res,
        }
    }
}

/// Mimicking `core::result::Result`, note this has a third `Opaque` variant
/// that enables dagtime variance
#[must_use]
#[derive(Debug, Clone, Copy)]
pub enum Result<T, E> {
    Ok(T),
    Err(E),
    Opaque(OpaqueInternal<T, E>),
}

use crate::mimick::Result::Opaque;
pub use crate::mimick::Result::{Err, Ok};

impl<T, E> From<awi::Result<T, E>> for dag::Result<T, E> {
    fn from(value: awi::Result<T, E>) -> Self {
        match value {
            awi::Ok(t) => Ok(t),
            awi::Err(e) => Err(e),
        }
    }
}

impl<T, E> Result<T, E> {
    pub fn ok_at_dagtime(t: T, is_ok: dag::bool) -> Self {
        Opaque(OpaqueInternal {
            is_ok,
            res: awi::Ok(t),
        })
    }

    pub fn err_at_dagtime(e: E, is_err: dag::bool) -> Self {
        Opaque(OpaqueInternal {
            is_ok: !is_err,
            res: awi::Err(e),
        })
    }

    pub fn as_ref(&self) -> Result<&T, &E> {
        match *self {
            Ok(ref t) => Ok(t),
            Err(ref e) => Err(e),
            Opaque(ref z) => Opaque(z.as_ref()),
        }
    }

    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> {
        match *self {
            Ok(ref mut t) => Ok(t),
            Err(ref mut e) => Err(e),
            Opaque(ref mut z) => Opaque(z.as_mut()),
        }
    }

    pub fn as_deref(&self) -> Result<&<T as Deref>::Target, &E>
    where
        T: Deref,
    {
        match self.as_ref() {
            Ok(t) => Ok(t),
            Err(e) => Err(e),
            // need to write this out for some reason
            Opaque(z) => Opaque(OpaqueInternal {
                is_ok: z.is_ok,
                res: match z.res {
                    StdOk(t) => StdOk(Deref::deref(t)),
                    StdErr(e) => StdErr(e),
                },
            }),
        }
    }

    pub fn as_deref_mut(&mut self) -> Result<&mut <T as Deref>::Target, &mut E>
    where
        T: DerefMut,
    {
        match self.as_mut() {
            Ok(t) => Ok(t),
            Err(e) => Err(e),
            // need to write this out for some reason
            Opaque(z) => Opaque(OpaqueInternal {
                is_ok: z.is_ok,
                res: match z.res {
                    StdErr(e) => StdErr(e),
                    StdOk(t) => StdOk(DerefMut::deref_mut(t)),
                },
            }),
        }
    }

    pub fn copied(self) -> Result<T, E>
    where
        T: Copy,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e),
            Opaque(z) => Opaque(z.copied()),
        }
    }

    pub fn cloned(self) -> Result<T, E>
    where
        T: Clone,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e),
            Opaque(z) => Opaque(z.cloned()),
        }
    }

    #[must_use]
    pub fn is_ok_at_runtime(&self) -> bool {
        match self {
            Ok(_) => true,
            Err(_) => false,
            Opaque(_) => false,
        }
    }

    #[must_use]
    pub fn is_ok(&self) -> dag::bool {
        match self {
            Ok(_) => true.into(),
            Err(_) => false.into(),
            Opaque(z) => z.is_ok,
        }
    }

    #[must_use]
    pub fn is_err_at_runtime(&self) -> bool {
        match self {
            Ok(_) => false,
            Err(_) => true,
            Opaque(_) => false,
        }
    }

    #[must_use]
    pub fn is_err(&self) -> dag::bool {
        match self {
            Ok(_) => false.into(),
            Err(_) => true.into(),
            Opaque(z) => !z.is_ok,
        }
    }

    #[must_use]
    pub fn is_opaque_at_runtime(&self) -> bool {
        match self {
            Ok(_) => false,
            Err(_) => false,
            Opaque(_) => false,
        }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Result<U, E> {
        match self {
            Ok(t) => Ok(f(t)),
            Err(e) => Err(e),
            Opaque(z) => Opaque(OpaqueInternal {
                is_ok: z.is_ok,
                res: z.res.map(f),
            }),
        }
    }

    #[must_use]
    pub fn ok(self) -> dag::Option<T> {
        match self {
            Ok(t) => dag::Some(t),
            Err(_) => dag::None,
            Opaque(z) => dag::Option::Opaque(crate::mimick::option::OpaqueInternal {
                is_some: z.is_ok,
                t: match z.res {
                    StdOk(t) => awi::Some(t),
                    StdErr(_) => awi::None,
                },
            }),
        }
    }

    #[must_use]
    pub fn err(self) -> dag::Option<E> {
        match self {
            Ok(_) => dag::None,
            Err(e) => dag::Some(e),
            Opaque(z) => dag::Option::Opaque(crate::mimick::option::OpaqueInternal {
                is_some: !z.is_ok,
                t: match z.res {
                    StdOk(_) => awi::None,
                    StdErr(e) => awi::Some(e),
                },
            }),
        }
    }

    #[track_caller]
    pub fn unwrap_at_runtime(self) -> T
    where
        E: Debug,
    {
        match self {
            Ok(t) => t,
            Err(e) => panic!("called `Result::unwrap_at_runtime()` on an `Err` value: {e:?}"),
            Opaque(_) => panic!("called `Result::unwrap_at_runtime()` on an `Opaque` value"),
        }
    }

    #[track_caller]
    pub fn unwrap(self) -> T
    where
        E: Debug,
    {
        match self {
            Ok(t) => t,
            Err(e) => panic!("called `Result::unwrap()` on an `Err` value: {e:?}"),
            Opaque(z) => {
                let tmp = std::panic::Location::caller();
                let location = Location {
                    file: tmp.file(),
                    line: tmp.line(),
                    col: tmp.column(),
                };
                register_assertion_bit_for_current_epoch(z.is_ok, location);
                if let StdOk(t) = z.res {
                    t
                } else {
                    panic!("called `Result::unwrap()` on an error-type `Opaque` value")
                }
            }
        }
    }

    #[track_caller]
    pub fn unwrap_err(self) -> E
    where
        T: Debug,
    {
        match self {
            Ok(t) => panic!("called `Result::unwrap_err()` on an `Ok` value: {t:?}"),
            Err(e) => e,
            Opaque(z) => {
                let tmp = std::panic::Location::caller();
                let location = Location {
                    file: tmp.file(),
                    line: tmp.line(),
                    col: tmp.column(),
                };
                register_assertion_bit_for_current_epoch(!z.is_ok, location);
                if let StdErr(e) = z.res {
                    e
                } else {
                    panic!("called `Result::unwrap()` on an ok-type `Opaque` value")
                }
            }
        }
    }
}

impl<T: Borrow<Bits> + BorrowMut<Bits>, E> Result<T, E> {
    #[track_caller]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Ok(t) => t,
            Err(_) => default,
            Opaque(z) => match z.res {
                StdOk(mut t) => {
                    if t.borrow_mut()
                        .mux_(default.borrow(), z.is_ok)
                        .is_none_at_runtime()
                    {
                        panic!("called `Result::unwrap_or()` with unequal bitwidth types")
                    }
                    t
                }
                StdErr(_) => {
                    panic!("called `Result::unwrap_or()` with error-type `Opaque`")
                }
            },
        }
    }
}

impl<T, E> Termination for Result<T, E> {
    fn report(self) -> ExitCode {
        match self {
            Ok(_) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
            Opaque(z) => match z.res {
                StdOk(_) => ExitCode::SUCCESS,
                StdErr(_) => ExitCode::FAILURE,
            },
        }
    }
}

#[cfg(feature = "try_support")]
impl<T, E> std::ops::Residual<T> for Result<!, E> {
    type TryType = Result<T, E>;
}

#[cfg(feature = "try_support")]
impl<T, E, F: From<E>> std::ops::FromResidual<Result<!, E>> for Result<T, F> {
    fn from_residual(residual: Result<!, E>) -> Self {
        match residual {
            Err(e) => Err(From::from(e)),
            _ => unreachable!(),
        }
    }
}

#[cfg(feature = "try_support")]
impl<T, E> std::ops::Try for Result<T, E> {
    type Output = T;
    type Residual = Result<!, E>;

    fn from_output(output: Self::Output) -> Self {
        Ok(output)
    }

    #[track_caller]
    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        use std::ops::ControlFlow;
        match self {
            Ok(t) => ControlFlow::Continue(t),
            Err(e) => ControlFlow::Break(Err(e)),
            Opaque(z) => match z.res {
                StdOk(t) => {
                    let tmp = std::panic::Location::caller();
                    let location = Location {
                        file: tmp.file(),
                        line: tmp.line(),
                        col: tmp.column(),
                    };
                    register_assertion_bit_for_current_epoch(z.is_ok, location);
                    ControlFlow::Continue(t)
                }
                StdErr(e) => {
                    let tmp = std::panic::Location::caller();
                    let location = Location {
                        file: tmp.file(),
                        line: tmp.line(),
                        col: tmp.column(),
                    };
                    register_assertion_bit_for_current_epoch(!z.is_ok, location);
                    ControlFlow::Break(Err(e))
                }
            },
        }
    }
}
