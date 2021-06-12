#![cfg(test)]

use crate::code_gen;

macro_rules! failures {
    ($($input:expr, $error:expr);*;) => {
        $(
            assert_eq!(code_gen(&$input, true, true), Err($error.to_owned()));
        )*
    };
}

#[test]
fn failures() {
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
