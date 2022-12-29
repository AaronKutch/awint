use std::{
    fmt::Write,
    fs::{File, OpenOptions},
};

use awint::awint_macro_internals::{cc_macro, CodeGen, FnNames, AWINT_NAMES};

const TEST_FN_NAMES: FnNames = FnNames {
    get_bw: "bw",
    mut_bits_ref: "&mut B",
    bits_ref: "&B",
    usize_add: "add",
    usize_sub: "sub",
    max_fn: "max",
    cc_checks_fn: "check_fn",
    copy_: "copy",
    field: "field",
    field_to: "field_to",
    field_from: "field_from",
    field_width: "field_width",
    field_bit: "field_bit",
    bw_call: &['.', 'b', 'w', '(', ')'],
};

fn cc(mut f: impl Write, input: &str) {
    let code_gen = CodeGen {
        static_width: false,
        return_type: None,
        must_use: |s| format!("mu({s})"),
        lit_construction_fn: |awi| format!("lit({awi})"),
        construction_fn: |s, w, d| {
            format!(
                "awi({},{:?},{:?})",
                if s.is_empty() { "zero" } else { s },
                w,
                d
            )
        },
        fn_names: TEST_FN_NAMES,
    };
    match cc_macro(input, code_gen, AWINT_NAMES) {
        Ok(s) => {
            writeln!(f, "{input}\nOk:\n{s}\n\n").unwrap();
        }
        Err(e) => {
            writeln!(f, "{input}\nErr:\n{e}\n\n").unwrap();
        }
    }
}

fn static_cc(mut f: impl Write, input: &str) {
    let code_gen = CodeGen {
        static_width: true,
        return_type: Some("StaticAwi"),
        must_use: |s| format!("mu({s})"),
        lit_construction_fn: |awi| format!("lit({awi})"),
        construction_fn: |s, w, d| {
            format!(
                "awi({},{:?},{:?})",
                if s.is_empty() { "zero" } else { s },
                w,
                d
            )
        },
        fn_names: TEST_FN_NAMES,
    };
    match cc_macro(input, code_gen, AWINT_NAMES) {
        Ok(s) => {
            writeln!(f, "{input}\nOk:\n{s}\n\n").unwrap();
        }
        Err(e) => {
            writeln!(f, "{input}\nErr:\n{e}\n\n").unwrap();
        }
    }
}

fn dynamic_cc(mut f: impl Write, input: &str) {
    let code_gen = CodeGen {
        static_width: false,
        return_type: Some("DynamicAwi"),
        must_use: |s| format!("mu({s})"),
        lit_construction_fn: |awi| format!("lit({awi})"),
        construction_fn: |s, w, d| {
            format!(
                "awi({},{:?},{:?})",
                if s.is_empty() { "zero" } else { s },
                w,
                d
            )
        },
        fn_names: TEST_FN_NAMES,
    };
    match cc_macro(input, code_gen, AWINT_NAMES) {
        Ok(s) => {
            writeln!(f, "{input}\nOk:\n{s}\n\n").unwrap();
        }
        Err(e) => {
            writeln!(f, "{input}\nErr:\n{e}\n\n").unwrap();
        }
    }
}

fn main() {
    let mut s = "Generated by `macro_outputs.rs`. Note: see the vt100-syntax-highlighting vscode \
                 extension and select preview on the file.\n\n"
        .to_owned();

    // all errors

    // initial parsing
    cc(&mut s, "0(1[2{34)5]6}7");
    cc(&mut s, "xx[[x(((x)))x]]x}}x");
    cc(&mut s, "x{x[x(((x)))x]]]x}x");
    cc(&mut s, "x{x[[x((x))))x]]x}x");
    cc(&mut s, "x,,");
    cc(&mut s, "x,;,");
    cc(&mut s, "x,;;");
    cc(&mut s, "");

    // component level
    cc(&mut s, "zero: , x; x");
    cc(&mut s, "[..]");
    cc(&mut s, "0u0");
    cc(&mut s, "-123");
    // this follows normal rust rules
    cc(&mut s, "x[..-1]");
    cc(&mut s, "x[-1..]");
    cc(&mut s, "x[..(-1)]");
    cc(&mut s, "x[(-1)..]");
    static_cc(&mut s, "0x123u12[7 + j..i + 5]");
    cc(&mut s, "x[1..0]");
    cc(&mut s, "x[(r+1)..r]");
    cc(&mut s, "x[1..1]");
    cc(&mut s, "x[r..r]");
    cc(&mut s, "0u8[8..]");
    cc(&mut s, "0u8[..9]");
    cc(&mut s, "0u8[r..(r + 9)]");
    cc(&mut s, "1..");
    cc(&mut s, "var[]");
    cc(&mut s, "x[0..1..2]");
    cc(&mut s, "x[0..=1..=2]");
    cc(&mut s, "x[0...1]");
    cc(&mut s, "a,b,c;d,e,0..0,f;g,h,i");
    static_cc(
        &mut s,
        "a, b, c; 0x1u4, 0x2u4, 0, 0x3u4; a[r..(r + 3)], b, c;",
    );

    // concatenation level
    cc(&mut s, "x; 0u8");
    cc(&mut s, "x; ..");
    cc(&mut s, ".., ..");
    cc(&mut s, "x[r..(r+7)]; x[(r-7)..(r+1)]");
    dynamic_cc(&mut s, ".., x");
    static_cc(&mut s, ".., x");
    static_cc(&mut s, "x");
    dynamic_cc(&mut s, "zero: .., x");
    dynamic_cc(&mut s, "zero: x, .., y; .., w");
    dynamic_cc(&mut s, "zero: .., x; y, ..");

    // successes

    static_cc(&mut s, "0x123u12");
    static_cc(&mut s, "-0xabcd1234i36");
    static_cc(&mut s, "0x123u12[4..8]");
    static_cc(&mut s, "0xau4, 0x4321u32[8..12], 0x7u4");
    static_cc(&mut s, "0x123u12[i]");
    static_cc(&mut s, "var[i][j][k]");
    static_cc(&mut s, " var [ 3 ..= 7 ] ");
    dynamic_cc(&mut s, "0x123u12[(7 + j)..(i + 5)]");
    dynamic_cc(&mut s, "0x123u12[..(i - j + 5)]");
    static_cc(&mut s, "0x123u12[(i - 1)..(i + 5)]");
    static_cc(&mut s, "0x123u12[(+ 5 + i - j)..(var - 7)]; ..64");
    static_cc(&mut s, "0x123u12[(+ 5 + i - j)..(var - 7)]; ..8");
    dynamic_cc(&mut s, "x[..(r as usize)]");
    dynamic_cc(&mut s, "x[..(r)]");
    dynamic_cc(&mut s, "x[..((r - lo) as usize)]");
    dynamic_cc(
        &mut s,
        "umax: 0xau4, b, 0xcu4, .., 0x98765_u20[x..(sink0.bw() - 9)], e, 0xfu4; sink0; sink1,;",
    );
    dynamic_cc(&mut s, "zero: .., x; .., y");
    static_cc(
        &mut s,
        "x; .., a[..(x.bw())], b[..y]; .., c[..z], d[..w]; ..128",
    );
    cc(&mut s, "..8, 0x1111u16, ..8; a, b; c, d;");
    dynamic_cc(&mut s, "uone:  ..=, ; ..=18, ..e, ..=, ");
    dynamic_cc(
        &mut s,
        "umax: 0xau4, b, 0xcu4, .., 0xdu4, e, 0xfu4; sink0; sink1",
    );
    cc(&mut s, "imax: 0..=1, 0x0_u1[0..1], x[..=], 0..=r, ..3; y");
    cc(&mut s, "zero: ..; .., x[..r0]; .., y[..r1]");
    // note: this is slightly different from an earlier one
    dynamic_cc(&mut s, "imax: ..; .., x; .., y");
    dynamic_cc(&mut s, "imin: ..r");
    dynamic_cc(&mut s, "imin: ..8");
    dynamic_cc(&mut s, "imin: y");
    static_cc(&mut s, "imin: y; ..8");
    cc(&mut s, "imin: y");

    let mut f = OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open("./testcrate/assets/macro_outputs.vt100")
        .unwrap();
    <File as std::io::Write>::write_all(&mut f, s.as_bytes()).unwrap();
}
