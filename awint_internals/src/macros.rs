//! Macros for export

/// A basic for loop for const contexts
#[macro_export]
macro_rules! const_for {
    ($i:ident in $range:block $b:block) => {
        let mut $i: usize = $range.start.wrapping_sub(1);
        loop {
            // the increment must happen before `$b` so that `continue`s still cause it
            $i = $i.wrapping_add(1);
            if $i >= $range.end {
                break
            }
            $b;
        }
    };
    ($i:ident in $range:block.rev() $b:block) => {
        let mut $i: usize = $range.end;
        loop {
            if $i <= $range.start {
                break
            }
            $i = $i.wrapping_sub(1);
            $b;
        }
    };
}

/// `f(x)` is run on every digit from first to last.
#[macro_export]
macro_rules! unsafe_for_each {
    ($lhs:ident, $x:ident, $f:block) => {
        unsafe {
            // Safety: This accesses all regular digits within their bounds
            const_for!(i in {0..$lhs.total_digits()} {
                let $x = $lhs.get_unchecked(i);
                $f;
            });
        }
    };
    ($lhs:ident, $x:ident, $range:block $f:block) => {
        unsafe {
            // Safety: This accesses all regular digits within their bounds
            const_for!(i in $range {
                let $x = $lhs.get_unchecked(i);
                $f;
            });
        }
    };
}

/// `f(x)` is run on every digit from first to last.
#[macro_export]
macro_rules! unsafe_for_each_mut {
    ($lhs:ident, $x:ident, $f:block, $clear_unused_bits:expr) => {
        unsafe {
            // Safety: This accesses all regular digits within their bounds
            const_for!(i in {0..$lhs.total_digits()} {
                let $x = $lhs.get_unchecked_mut(i);
                $f;
            });
        }
        if $clear_unused_bits {
            $lhs.clear_unused_bits()
        }
    };
    ($lhs:ident, $x:ident, $range:block $f:block, $clear_unused_bits:expr) => {
        unsafe {
            // Safety: This accesses all regular digits within their bounds
            const_for!(i in $range {
                let $x = $lhs.get_unchecked_mut(i);
                $f;
            });
        }
        if $clear_unused_bits {
            $lhs.clear_unused_bits()
        }
    };
}

/// If `lhs.bw() != rhs.bw()`, this returns `None`, otherwise `f(x, y)` is run
/// on every corresponding pair of digits from first to last.
#[macro_export]
macro_rules! unsafe_binop_for_each {
    ($lhs:ident, $rhs:ident, $x:ident, $y:ident, $f:block) => {
        if $lhs.bw() != $rhs.bw() {
            return None
        } else {
            unsafe {
                // Safety: This accesses all regular digits within their bounds. If the
                // bitwidths are equal, then the slice lengths are also equal.
                const_for!(i in {0..$lhs.total_digits()} {
                    let $x = $lhs.get_unchecked(i);
                    let $y = $rhs.get_unchecked(i);
                    $f;
                });
            }
            Some(())
        }
    };
    ($lhs:ident, $rhs:ident, $x:ident, $y:ident, $range:block .rev() $f:block) => {
        if $lhs.bw() != $rhs.bw() {
            return None
        } else {
            unsafe {
                // Safety: This accesses all regular digits within their bounds. If the
                // bitwidths are equal, then the slice lengths are also equal.
                const_for!(i in $range.rev() {
                    let $x = $lhs.get_unchecked(i);
                    let $y = $rhs.get_unchecked(i);
                    $f;
                });
            }
            Some(())
        }
    };
    ($lhs:ident, $rhs:ident, $x:ident, $y:ident, $preloop:block, $range:block .rev() $f:block) => {
        if $lhs.bw() != $rhs.bw() {
            return None
        } else {
            $preloop
            unsafe {
                // Safety: This accesses all regular digits within their bounds. If the
                // bitwidths are equal, then the slice lengths are also equal.
                const_for!(i in $range.rev() {
                    let $x = $lhs.get_unchecked(i);
                    let $y = $rhs.get_unchecked(i);
                    $f;
                });
            }
            Some(())
        }
    };
}

/// If `lhs.bw() != rhs.bw()`, this returns `None`, otherwise `f(x, y)` is run
/// on every corresponding pair of digits from first to last.
#[macro_export]
macro_rules! unsafe_binop_for_each_mut {
    ($lhs:ident, $rhs:ident, $x:ident, $y:ident, $f:block, $clear_unused_bits:expr) => {
        if $lhs.bw() != $rhs.bw() {
            return None
        } else {
            unsafe {
                // Safety: This accesses all regular digits within their bounds. If the
                // bitwidths are equal, then the slice lengths are also equal.
                const_for!(i in {0..$lhs.total_digits()} {
                    let $x = $lhs.get_unchecked_mut(i);
                    let $y = $rhs.get_unchecked(i);
                    $f;
                });
            }
            if $clear_unused_bits {
                $lhs.clear_unused_bits()
            }
            Some(())
        }
    };
}

/// Runs `f` on a digitwise subslice `subbits` of `bits`. This is a macro
/// because closures are not properly supported in `const` functions yet.
///
/// # Safety
///
/// `range` must satisfy `range.start <= range.end` and `range.end <=
/// self.total_digits()`
#[macro_export]
macro_rules! subdigits_mut {
    ($bits:ident, $range:expr, $subbits:ident, $f:block) => {
        // because this macro is especially unsafe, do not inlude
        // an `unsafe` block here and make the caller handle it.
        debug_assert!($range.start <= $range.end);
        debug_assert!($range.end <= $bits.total_digits());
        // prevent a zero bitwidth
        if $range.start != $range.end {
            // Safety: This maintains the invariants of `Bits`. This works even
            // if the range is a full range. The range is nonempty.

            let $subbits: &mut Bits = Bits::internal_subdigits_mut($bits, $range.start, $range.end);
            // then run the "closure" on the fixed subslice
            $f
            // make double sure that the reference is not used again
            #[allow(unused_variables)]
            let $subbits = ();
        }
    }
}

#[macro_export]
macro_rules! forward_debug_fmt {
    ($name:ident) => {
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Debug::fmt(self, f)
            }
        }

        impl fmt::LowerHex for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Debug::fmt(self, f)
            }
        }

        impl fmt::UpperHex for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Debug::fmt(self, f)
            }
        }

        impl fmt::Octal for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Debug::fmt(self, f)
            }
        }

        impl fmt::Binary for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Debug::fmt(self, f)
            }
        }
    };
}

/// Returns the `Location` at the first layer of macros this is in
#[macro_export]
macro_rules! location {
    () => {
        $crate::Location {
            file: file!(),
            line: line!(),
            col: column!(),
        }
    };
}
