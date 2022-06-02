//! main lowering logic
//!
//! There are some things to explain about the fielding implementation. If we
//! tried to do something like ORing all the boundaries of the source and sink
//! together, and then fielding all those regions, it would lead to arbitrarily
//! complex logic because dynamic ranges can push the components to align with
//! multiple different components in the sink, and slide to completely different
//! places at runtime. In other words, if all ranges are dynamic then every
//! fielding operation would need to be ready to field any component from the
//! source to any component in the sink. The other problem is that fielding
//! between references pointing to the same awi violates borrow rules. The only
//! reasonable solution is to have an intermediate buffer, to which the source
//! is fielded, then fielding from there to the sinks. This removes the need for
//! the fieldings only needing to know about one concatenation at a time and not
//! needing to account for both at the same time, and the complexity is kept
//! linear
//!
//! No-op fillers in sources adds one more layer of complexity. In most cases,
//! the bits in buffers corresponding to fillers are in a specified set state or
//! are never read from, but consider what happens in this case:
//!
//! (y is set to 0x2222u16 before the operation)
//! `cc!(0x1111u16, ..8; ..8, y)`
//!
//! The output using the above approach would be `0x1100u16` instead of the
//! expected `0x1122u16` (assuming the buffer bits all start out as zeroed).
//! No-op source fillers effectively dictate that sink bits must be fielded to a
//! buffer, then non-filler source bits overwrite their respective parts, and
//! then the buffer gets written back to the sinks. Any other method runs into
//! the complexity multiplication problem, because the dynamic fielding to the
//! sink would need to be aware of all dynamic fillers in the source.

// TODO
// note: when refactoring keep first line of error (the "concatenation 0:
// component ...") mostly the same so error lens works, add on more lines
// describing how the input is being parsed
//
// Known issues:
//
// Wrap fallible return values e.x.
// FIXME it is implemented as generic `Bits::must_use`
// so the compiler produces warnings
//
// Previously, we could not introduce the binding step into the macros, because
// we could not make two usecases work at the same time:
//
// ```
// // `a` is bound outside of the macro
// let mut a = inlawi!(0xau4);
// { // inside macro
//     let tmp_bind = a.const_as_mut();
//     // ...
// }
// // use a for something later (if we did not call `const_as_mut`,
// // it would fail because `a` would be moved)
// ```
//
// ```
// // the macro is called directly with the constructor
// {
//     // error[E0716]: temporary value dropped while borrowed
//     let tmp_bind = inlawi!(0xau4).const_as_mut();
//     // ...
// }
// ```
//
// I discovered that for whatever reason, some builtin traits such as `AsRef`
// and `AsMut` avoid E0716 (be sure to explicitly include the reference type to
// reduce reference nesting if the bound part is external stuff that already has
// a layer of reference):
// ```
// let __awint_bind_0: &Bits = &inlawi!(0xau4);
// let __awint_bind_1: &mut Bits = &mut inlawi!(0xau4);
// ```
//
// Make default initializations be postfixed with ':' to reduce macro
// duplication. The only thing this will prevent is certain complicated
// expressions.
//
// Some expressions such as `((rbb.0 - lo) as usize)` get broken because of
// space removal
//
// `inlawi!(x[..1])` and other guaranteed width>=1 && (shl=0 || shl=bw-1) parts
// should be infallible
//
// There needs to be an initial typeless binding e.x. `let __awint_bind_0 =
// f(x)` so that complicated expressions are not called twice for their
// bitwidths and getting references from them. This also prevents lifetimes
// being too short from intermediates.
//
// In some cases (e.x. `inlawi_imin(5..7)`), certain width values and shift
// increments are created when they are not needed. However, the known cases
// are very easily optimized away (I _think_ they are already optimized away
// at the MIR level before progressing). The code is complicated enough as it
// is, perhaps this should be fixed in a future refactor.
//
// The static width determination system isn't smart enough to know that
// "x[pos..=pos]" or "x[pos]" has a bitwidth of 1. The docs say that same
// string inputs should not be generator like, and the code gen already
// removes redundant value, so we should handle this in `Usbr` somewhere.
//
// The range value parser should be able to handle hexadecimal and octal
// statically (e.x. `x[0x10..0x15]` should have known bitwidth).
//
// TODO: document new hexadecimal, octal, binary, and decimal parsing,
// some things optimization can do (but nesting still works),
// that an extra index like x[i][..], x[i][12..42] can coexist,
// that nested invocations and most Rust syntax should be able to work,

use std::{fmt::Write, num::NonZeroUsize};

use awint_ext::ExtAwi;

use crate::{Ast, Bind, CodeGen, ComponentType::*, EitherResult, Lower, Names};

/// Lowering of the parsed structs into Rust code.
pub fn cc_macro_code_gen<
    F0: FnMut(&str) -> String,
    F1: FnMut(ExtAwi) -> String,
    F2: FnMut(&str, Option<NonZeroUsize>, Option<&str>) -> String,
>(
    mut ast: Ast,
    specified_init: bool,
    mut code_gen: CodeGen<'_, F0, F1, F2>,
    names: Names,
) -> String {
    // first check for simple infallible constant return
    if code_gen.return_type.is_some() && (ast.cc.len() == 1) && (ast.cc[0].comps.len() == 1) {
        let comp = &ast.cc[0].comps[0];
        if let Literal(ref lit) = comp.c_type {
            // constants have been normalized and combined by now
            if comp.range.static_range().is_some() {
                return (code_gen.must_use)(&(code_gen.lit_construction_fn)(ExtAwi::from_bits(lit)))
            }
        }
    }

    let fn_names = code_gen.fn_names;
    let mut l = Lower::new(names, fn_names);

    let mut need_buffer = false;
    let mut source_has_filler = false;

    for (concat_i, concat) in ast.cc.iter_mut().enumerate() {
        if (concat.comps.len() != 1) || (!concat.comps[0].has_full_range()) {
            need_buffer = true;
        }
        for comp in &mut concat.comps {
            match comp.c_type {
                Unparsed => unreachable!(),
                Literal(ref awi) => {
                    comp.bind = Some(
                        l.binds
                            .insert(Bind::Literal(awi.clone()), (false, false))
                            .either(),
                    )
                }
                Variable => {
                    comp.bind = Some(l.binds.insert(Bind::Txt(comp.txt), (false, false)).either())
                }
                Filler => {
                    if concat_i == 0 {
                        source_has_filler = true;
                    }
                }
            }
        }
    }

    // work backwards so we calculate only what we need
    for concat in &mut ast.cc {
        l.lower_concat(concat);
    }
    let lt_checks = l.lower_lt_checks();
    let common_lt_checks = l.lower_common_lt_checks(&ast);
    let common_ne_checks = l.lower_common_ne_checks(&ast);
    let infallible =
        lt_checks.is_empty() && common_lt_checks.is_empty() && common_ne_checks.is_empty();

    let construction = if code_gen.return_type.is_some() || need_buffer {
        // FIXME separate construction and buffer?
        format!(
            "let mut {} = {};\n",
            names.awi,
            (code_gen.construction_fn)("", ast.common_bw, Some(names.cw))
        )
    } else {
        String::new()
    };

    let fielding = l.lower_fielding(&ast, source_has_filler, specified_init, need_buffer);

    let returning = match (code_gen.return_type.is_some(), infallible) {
        (false, false) => "Some(())".to_owned(),
        (false, true) => String::new(),
        (true, false) => (code_gen.must_use)(&format!("Some({})", names.awi)),
        (true, true) => (code_gen.must_use)(names.awi),
    };

    // inner code consisting of the zero check, construction of returning or
    // buffers, fielding, and return values
    let mut inner0 = format!("{}\n{}\n{}", construction, fielding, returning);
    if !infallible {
        if code_gen.return_type.is_some() {
            // checking if common width is zero
            inner0 = format!("if {} != 0 {{\n{}\n}} else {{None}}", names.cw, inner0);
        } else {
            // Non-construction macros can have a zero concatenation bitwidth, but we have
            // to avoid creating the buffer.
            inner0 = format!("if {} != 0 {{\n{}\n}} else {{Some(())}}", names.cw, inner0);
        }
    }

    // designate the common concatenation width
    let common_cw = if let Some(bw) = ast.common_bw {
        format!("let {} = {}usize;\n", names.cw, bw)
    } else if let Some(p_sum_width) = l.dynamic_width {
        let s = format!(
            "let {} = {}_{};\n",
            names.cw,
            names.cw,
            p_sum_width.get_raw()
        );
        s
    } else {
        // for the case with all unbounded fillers, find the max bitwidth for the buffer
        // to use.
        let mut s = String::new();
        for concat in &ast.cc {
            if !s.is_empty() {
                s += ",";
            }
            write!(s, "{}_{}", names.cw, concat.cw.unwrap().get_raw()).unwrap();
        }
        format!("let {} = {}({});\n", fn_names.max_fn, names.cw, s)
    };

    // common width calculation comes before the zero check
    let inner1 = format!("{}\n{}", common_cw, inner0);

    let cws = l.lower_cws();
    let widths = l.lower_widths();

    // width and common width calculations come after range checks and before equal
    // width checks
    let inner2 = if common_ne_checks.is_empty() {
        inner1
    } else {
        format!("if {} {{\n{}\n}} else {{None}}", common_ne_checks, inner1)
    };

    let inner3 = format!("{}\n{}\n{}", widths, cws, inner2);

    // range checks
    let inner4 = if common_lt_checks.is_empty() {
        inner3
    } else {
        format!("if {} {{\n{}\n}} else {{None}}", common_lt_checks, inner3)
    };

    let values = l.lower_values();
    let bindings = l.lower_bindings(&ast, code_gen.lit_construction_fn);

    format!("{{{}\n{}\n{}}}", bindings, values, inner4)
}
