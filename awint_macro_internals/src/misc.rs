use std::num::{NonZeroU32, NonZeroUsize};

use awint_ext::{Awi, Bits};
use triple_arena::ptr_struct;

//ptr_struct!(PText; PBind; PVal; PWidth; PCWidth);

// we should never need 4 billion entries for these macros
ptr_struct!(
    PText[NonZeroU32]();
    PBind[NonZeroU32]();
    PVal[NonZeroU32]();
    PWidth[NonZeroU32]();
    PCWidth[NonZeroU32]()
);

pub fn i128_to_usize(x: i128) -> Result<usize, String> {
    usize::try_from(x).map_err(|_| "`usize::try_from` overflow".to_owned())
}

pub fn i128_to_nonzerousize(x: i128) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(i128_to_usize(x)?).ok_or_else(|| "`NonZeroUsize::new` overflow".to_owned())
}

pub fn usize_to_i128(x: usize) -> Result<i128, String> {
    i128::try_from(x).map_err(|_| "`i128::try_from` overflow".to_owned())
}

pub fn chars_to_string(chars: &[char]) -> String {
    let mut s = String::new();
    for c in chars {
        s.push(*c);
    }
    s
}

pub fn awint_must_use(s: &str) -> String {
    format!("Bits::must_use({s})")
}

/// Returns architecture-independent Rust code that returns an
/// `InlAwi` type with bitwidth `w`.
pub fn unstable_native_inlawi_ty(w: u128) -> String {
    format!("InlAwi::<{w},{{Bits::unstable_raw_digits({w})}}>")
}

/// Returns architecture-independent Rust code that returns an
/// `InlAwi` preset with the value of `bits`.
pub fn unstable_native_inlawi(bits: &Bits) -> String {
    // gets `bits` in `Vec<u8>` form, truncated
    let sig = bits.sig();
    let len = (sig / 8) + (((sig % 8) != 0) as usize);
    let mut buf = vec![0u8; len];
    bits.to_u8_slice(&mut buf);

    // this absolutely has to be done, because the proc-macro crate may be run on an
    // architecture with a different pointer width than the true target architecture
    // (and there is currently no way to get information about the true target
    // architecture from the build architecture, in fact the same output could
    // potentially be used on multiple architectures). `unstable_raw_digits` adjusts
    // `LEN` based on the native `usize` width, and `unstable_from_u8_slice` also
    // adjusts for big endian archiectures.

    // note: it might be optimal in certain cases to make the `InlAwi` itself const,
    // this would lead to being able to directly memcpy it. However, if we make the
    // &[u8] level const then it allows for certain unsigned constants to be shared
    // in .text even if in different sized integers and makes them smaller for
    // things with lots of leading zeros. Also, we can't make the `InlAwi` level
    // const anyway because of `awint_dag`.
    format!(
        "InlAwi::<{},{{Bits::unstable_raw_digits({})}}>::unstable_from_u8_slice({{const B: \
         &[core::primitive::u8] = &{:?}; B}})",
        bits.bw(),
        bits.bw(),
        buf,
    )
}

// there is some strange issue with feature flags through proc-macro crates that
// prevents this from working
/*#[cfg(not(feature = "const_support"))]
pub fn unstable_native_bits(bits: &Bits) -> String {
    format!("{{let b: &Bits = &{}; b}}", unstable_native_inlawi(bits))
}*/

// Returns architecture-independent Rust code that returns a `&'static Bits`
// equal to `bits`.
//#[cfg(feature = "const_support")]
pub fn unstable_native_bits(bits: &Bits) -> String {
    format!("{{const B: &Bits = &{}; B}}", unstable_native_inlawi(bits))
}

// Originally, this was going to use `unstable_native_bits`, however:
//
// - This will currently require virtually all crates to use
//   `#![feature(const_trait_impl)]` and some others unconditionally
// - If the same constant is used several times but with different leading zeros
//   or other surrounding data, it impacts .text reusability
// - More generated Rust code
//
// `unstable_native_bits` is now just used for `awint_bits_lit_construction_fn`
pub fn awint_static_construction_fn(awi: Awi) -> String {
    unstable_native_inlawi(&awi)
}

pub fn awint_unreachable_construction_fn(_awi: Awi) -> String {
    unreachable!()
}

pub fn awint_inlawi_lit_construction_fn(awi: Awi) -> String {
    unstable_native_inlawi(&awi)
}

pub fn awint_extawi_lit_construction_fn(awi: Awi) -> String {
    format!("ExtAwi::from_bits(&{})", unstable_native_inlawi(&awi))
}

pub fn awint_awi_lit_construction_fn(awi: Awi) -> String {
    format!("Awi::from_bits(&{})", unstable_native_inlawi(&awi))
}

pub fn awint_bits_lit_construction_fn(awi: Awi) -> String {
    unstable_native_bits(&awi)
}

pub fn inlawi_s(init: &str, w: NonZeroUsize) -> String {
    format!("InlAwi::<{w},{{Bits::unstable_raw_digits({w})}}>::{init}()",)
}

pub fn extawi_s(init: &str, s: &str) -> String {
    format!("ExtAwi::panicking_{init}({s})")
}

pub fn awi_s(init: &str, s: &str) -> String {
    format!("Awi::panicking_{init}({s})")
}

pub fn cc_construction_fn(
    mut init: &str,
    static_width: Option<NonZeroUsize>,
    dynamic_width: Option<&str>,
) -> String {
    if init.is_empty() {
        init = "zero";
    }
    if let Some(w) = static_width {
        inlawi_s(init, w)
    } else if let Some(s) = dynamic_width {
        awi_s(init, s)
    } else {
        unreachable!()
    }
}

pub fn inlawi_construction_fn(
    mut init: &str,
    static_width: Option<NonZeroUsize>,
    _dynamic_width: Option<&str>,
) -> String {
    if init.is_empty() {
        init = "zero";
    }
    if let Some(w) = static_width {
        inlawi_s(init, w)
    } else {
        unreachable!()
    }
}

pub fn extawi_construction_fn(
    mut init: &str,
    _static_width: Option<NonZeroUsize>,
    dynamic_width: Option<&str>,
) -> String {
    if init.is_empty() {
        init = "zero";
    }
    if let Some(s) = dynamic_width {
        extawi_s(init, s)
    } else {
        unreachable!()
    }
}

pub fn awi_construction_fn(
    mut init: &str,
    _static_width: Option<NonZeroUsize>,
    dynamic_width: Option<&str>,
) -> String {
    if init.is_empty() {
        init = "zero";
    }
    if let Some(s) = dynamic_width {
        awi_s(init, s)
    } else {
        unreachable!()
    }
}

pub fn identity_const_wrapper(
    inner: String,
    _w: Option<NonZeroUsize>,
    _infallible: bool,
) -> String {
    inner
}

pub fn awint_bits_const_wrapper(
    inner: String,
    w: Option<NonZeroUsize>,
    infallible: bool,
) -> String {
    let w = if let Some(w) = w {
        w
    } else {
        // this should only be used in a static width context
        unreachable!()
    };
    if infallible {
        format!(
            "{{const __B:{}={};\nconst __C:&Bits=&__B;__C}}",
            unstable_native_inlawi_ty(w.get() as u128),
            inner
        )
    } else {
        // the match is to avoid bringing in `const_option_ext`
        format!(
            "{{const __B:Option<{}>={};\nconst __C:Option<&Bits>=match __B {{Some(ref \
             b)=>Some(b),None=>None}};__C}}",
            unstable_native_inlawi_ty(w.get() as u128),
            inner
        )
    }
}
