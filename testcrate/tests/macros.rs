use awint::prelude::*;
use awint_macro_internals::code_gen;

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
            assert_eq!(code_gen(&$input, false, "zero", true, true), Err($error.to_owned()));
        )*
    };
}

#[test]
fn macro_failures() {
    failures!(
        // TODO This restriction could be lifted in the future
        "Ω", "concatenation 0 (\"Ω\"): component 0 (\"Ω\"): is not ascii";
        "", "input is empty or only whitespace";
        ";", "concatenation 0: is empty or only whitespace";
        ",", "concatenation 0 (\",\"): component 1: is empty or only whitespace";
        ";a", "concatenation 0: is empty or only whitespace";
        ",a", "concatenation 0 (\",a\"): component 1: is empty or only whitespace";
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
        "r..", "concatenation 0 (\"r..\"): component 0 (\"r..\"): A filler with a bounded start \
            should also have a bounded end";
        "..;a", "a construction macro with unspecified initialization cannot have a filler in the \
            source concatenation";
        "5..6;a", "a construction macro with unspecified initialization cannot have a filler in \
            the source concatenation";
        "a;..", "there is a sink concatenation that consists of only an unbounded filler";
        "a[..1];a[..2]", "determined statically that concatenations 0 and 1 have unequal bitwidths \
            1 and 2";
        "a", "`InlAwi` construction macros need at least one concatenation to have a width that \
            can be determined statically by the macro";
    );
    assert_eq!(
        code_gen(&"..".to_string(), true, "zero", false, false),
        Err(
            "there is a only a source concatenation that has no statically or dynamically \
             determinable width"
                .to_owned()
        )
    );
    assert_eq!(
        code_gen(&"a,..;b,..,c".to_string(), true, "zero", false, false),
        Err(
            "there is an unbounded filler in the middle of a concatenation, and no concatenation \
             has a statically or dynamically determinable width"
                .to_owned()
        )
    );
    assert_eq!(
        code_gen(&"a,..;..,b".to_string(), true, "zero", false, false),
        Err(
            "there are two concatenations with unbounded fillers aligned opposite each other, and \
             no concatenation has a statically or dynamically determinable width"
                .to_owned()
        )
    );
}

#[test]
fn macro_successes() {
    // both trailing comma and semicolon
    let _ = inlawi!(0u1,;);
    // basic concatenation
    assert_eq!(inlawi!(0xau4, 0x4321u16, 0x7u4), inlawi!(0xa43217u24));
    assert_eq!(inlawi!(0xau4, 0x4321u32[8..12], 0x7u4), inlawi!(0xa37u12));
    // copy assign
    let a = inlawi!(0xau4);
    let mut awi = ExtAwi::zero(bw(4));
    let b = awi.const_as_mut();
    let mut c = extawi!(0u4);
    cc!(a;b;c).unwrap();
    assert_eq!(a, inlawi!(0xau4));
    assert_eq!(a.const_as_ref(), b);
    assert_eq!(a.const_as_ref(), c.const_as_ref());
    // dynamic ranges
    let x: usize = 8;
    let awi = ExtAwi::zero(bw(12));
    assert_eq!(
        extawi!(0x98765_u20[x..(x + awi.bw())]).unwrap(),
        extawi!(0x987u12)
    );
    // unbounded fillers
    let mut sink0 = ExtAwi::zero(bw(44));
    let mut sink1 = ExtAwi::zero(bw(44));
    let b = inlawi!(0xbbu8);
    let e = inlawi!(0xeeeu12);
    let result = extawi!(0xabbcffdeeefu44);
    assert_eq!(
        extawi_umax!(0xau4, b, 0xcu4, .., 0xdu4, e, 0xfu4; sink0; sink1).unwrap(),
        result
    );
    assert_eq!(sink0, result);
    assert_eq!(sink1, result);
    let mut sink0 = extawi!(0xf0f0f0f0f0fu44);
    let mut sink1 = extawi!(0xf0f0f0f0f0fu44);
    let b = inlawi!(0xbbu8);
    let e = inlawi!(0xeeeu12);
    let result = extawi!(0xabbcf0deeefu44);
    cc!(0xau4, b, 0xcu4, .., 0xdu4, e, 0xfu4; sink0; sink1).unwrap();
    assert_eq!(sink0, result);
    assert_eq!(sink1, result);
    assert_eq!(extawi_umax!(..;..9), extawi_umax!(9));
    let a = inlawi!(0x123u12);
    let b = inlawi!(0x4u4);
    assert_eq!(extawi!(a, b), extawi!(0x1234u16));
    let a = inlawi!(0xau4);
    let mut b = inlawi!(0xbu4);
    let r0 = 0;
    let r1 = 4;
    assert_eq!(extawi!(a[..r0], b[..r1]), Some(extawi!(0xbu4)));
    assert_eq!(cc!(a[..r0]; b[..r0]), Some(()));
    assert_eq!(extawi!(a[..r0]), None);
    assert_eq!(cc!(r0..r1), Some(()));
    assert_eq!(inlawi!(0100[2]), inlawi!(1));
    assert_eq!(inlawi!(0100[3]), inlawi!(0));
    let r = 2;
    assert_eq!(extawi!(0100[r]).unwrap(), extawi!(1));
    let a = inlawi!(0xau4);
    let mut y = inlawi!(0u16);
    cc_imax!(.., a, ..4; y);
    assert_eq!(y, inlawi!(0x7fafu16));
}
