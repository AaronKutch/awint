use awint::prelude::*;
use awint_ext::internals::code_gen;

macro_rules! construction {
    ($($bw:expr)*) => {
        $(
            let inlawi = inlawi_zero!($bw);
            let extawi = ExtAwi::zero(bw($bw));
            assert!(inlawi.const_as_ref().is_zero());
            assert_eq!(inlawi.const_as_ref(), extawi.const_as_ref());
            let inlawi = inlawi_umax!($bw);
            let extawi = ExtAwi::umax(bw($bw));
            assert!(inlawi.const_as_ref().is_umax());
            assert_eq!(inlawi.const_as_ref(), extawi.const_as_ref());
            let inlawi = inlawi_imax!($bw);
            let extawi = ExtAwi::imax(bw($bw));
            assert!(inlawi.const_as_ref().is_imax());
            assert_eq!(inlawi.const_as_ref(), extawi.const_as_ref());
            let inlawi = inlawi_imin!($bw);
            let extawi = ExtAwi::imin(bw($bw));
            assert!(inlawi.const_as_ref().is_imin());
            assert_eq!(inlawi.const_as_ref(), extawi.const_as_ref());
            let inlawi = inlawi_uone!($bw);
            let extawi = ExtAwi::uone(bw($bw));
            assert!(inlawi.const_as_ref().is_uone());
            assert_eq!(inlawi.const_as_ref(), extawi.const_as_ref());
        )*
    };
}

#[test]
fn construction() {
    construction!(1 2 7 8 62 63 64 65 66 127 128 129 130 191 192 256 4096);
}

macro_rules! failures {
    ($($input:expr, $error:expr);*;) => {
        $(
            assert_eq!(code_gen(&$input, true, true), Err($error.to_owned()));
        )*
    };
}

#[test]
fn macro_failures() {
    failures!(
        // This restriction could be lifted in the future
        "Ω", "concatenation 0 (\"Ω\"): component 0 (\"Ω\"): is not ascii";
        "", "concatenation 0 (\"\"): component 0 (\"\"): is empty or only whitespace";
        " ", "concatenation 0 (\" \"): component 0 (\" \"): is empty or only whitespace";
        "a[1", "concatenation 0 (\"a[1\"): component 0 (\"a[1\"): has an opening '[' but not a \
            closing ']'";
        "a[]", "concatenation 0 (\"a[]\"): component 0 (\"a[]\"): has an empty index";
        "a[b..c..d]", "concatenation 0 (\"a[b..c..d]\"): component 0 (\"a[b..c..d]\"): too many \
            ranges";
        "a..b..c", "concatenation 0 (\"a..b..c\"): component 0 (\"a..b..c\"): too many ranges";
        " 0abc ", "concatenation 0 (\" 0abc \"): component 0 (\" 0abc \"): was parsed with \
            `<ExtAwi as FromStr>::from_str(\"0abc\")` which returned SerdeError::InvalidChar";
        "a;..,..", "concatenation 1 (\"..,..\"): there is more than one unbounded filler";
        "a; 0x1u1", "concatenation 1 (\" 0x1u1\"): sink concatenations cannot have literals";
        "a[..0]", "concatenation 0 (\"a[..0]\"): component 0 (\"a[..0]\"): determined statically \
            that this has zero bitwidth";
        "..;a", "a construction macro cannot have a filler in the source concatenation";
        "5..6;a", "a construction macro cannot have a filler in the source concatenation";
        "a;..", "concatenation 1 consists of only an unbounded filler";
        "a[..1];a[..2]", "determined statically that concatenations 0 and 1 have unequal bitwidths \
            1 and 2";
        "a", "InlAwi construction macros need at least one concatenation to have a width that can \
            be determined statically by the macro";
    );
    assert_eq!(
        code_gen(&"a,..;b,..,c".to_string(), false, false),
        Err(
            "there is an unbounded filler in the middle of a concatenation, and no concatenation \
             has a statically or dynamically determinable width"
                .to_owned()
        )
    );
    assert_eq!(
        code_gen(&"a,..;..,b".to_string(), false, false),
        Err(
            "there are two concatenations with unbounded fillers aligned opposite each other, and \
             no concatenation has a statically or dynamically determinable width"
                .to_owned()
        )
    );
}

#[test]
fn macro_successes() {
    assert_eq!(inlawi!(0xau4, 0x4321u16, 0x7u4), inlawi!(0xa43217u24));
    assert_eq!(inlawi!(0xau4, 0x4321u32[8..12], 0x7u4), inlawi!(0xa37u12));
    let a = inlawi!(0xau4);
    let mut awi = ExtAwi::zero(bw(4));
    let b = awi.const_as_mut();
    let mut c = extawi!(0u4);
    cc!(a;b;c).unwrap();
    assert_eq!(a, inlawi!(0xau4));
    assert_eq!(a.const_as_ref(), b);
    assert_eq!(a.const_as_ref(), c.const_as_ref());
}
