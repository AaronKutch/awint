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
            const_for!(i in {0..$lhs.len()} {
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
            const_for!(i in {0..$lhs.len()} {
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
                const_for!(i in {0..$lhs.len()} {
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
                const_for!(i in {0..$lhs.len()} {
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
/// self.len()`
#[macro_export]
macro_rules! subdigits_mut {
    ($bits:ident, $range:expr, $subbits:ident, $f:block) => {
        // because this macro is especially unsafe, do not inlude
        // an `unsafe` block here and make the caller handle it.
        debug_assert!($range.start <= $range.end);
        debug_assert!($range.end <= $bits.len());
        // prevent a zero bitwidth
        if $range.start != $range.end {
            // Safety: This maintains the metadata raw invariants of `Bits`. This works even
            // if the range is a full range. The range is nonempty.

            // when saving the metadata, we need to handle the edge case where `USIZE_BITS < BITS`
            if $crate::USIZE_BITS >= $crate::BITS {
                // We temporarily replace the digits needed for the subslice metadata
                let mut original = 0usize;
                const_for!(i in {0..$crate::METADATA_DIGITS} {
                    original |=
                        ($bits.as_ptr().add(i + $range.end).read() as usize) << (i * $crate::BITS);
                });
                // new metadata for the number of bits in our subdigits
                let mut metadata: usize = ($range.end - $range.start) * $crate::BITS;
                const_for!(i in {0..$crate::METADATA_DIGITS} {
                    *$bits.raw_get_unchecked_mut(i + $range.end) =
                        (metadata >> (i * $crate::BITS)) as $crate::Digit;
                });
                let $subbits: &mut Bits = Bits::from_raw_parts_mut(
                    $bits.as_mut_ptr().add($range.start),
                    ($range.end - $range.start) + $crate::METADATA_DIGITS,
                );
                // then run the "closure" on the fixed subslice
                $f
                // make sure that the reference is not used again
                #[allow(unused_variables)]
                let $subbits = ();
                // restore the subslice's bitwidth digit to whatever kind of digit it was in the
                // original slice
                const_for!(i in {0..$crate::METADATA_DIGITS} {
                    *$bits.raw_get_unchecked_mut(i + $range.end) =
                        (original >> (i * $crate::BITS)) as $crate::Digit;
                });
            } else {
                // We temporarily replace the digits needed for the subslice metadata
                let mut original: $crate::Digit = $bits.as_ptr().add($range.end).read();
                // new metadata for the number of bits in our subdigits
                let mut metadata = (($range.end - $range.start) * $crate::BITS) as $crate::Digit;
                *$bits.raw_get_unchecked_mut($range.end) = metadata;
                let $subbits: &mut Bits = Bits::from_raw_parts_mut(
                    $bits.as_mut_ptr().add($range.start),
                    ($range.end - $range.start) + $crate::METADATA_DIGITS,
                );
                // then run the "closure" on the fixed subslice
                $f
                // make sure that the reference is not used again
                #[allow(unused_variables)]
                let $subbits = ();
                // restore the subslice's bitwidth digit to whatever kind of digit it was in the
                // original slice
                *$bits.raw_get_unchecked_mut($range.end) = original;
            }
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
