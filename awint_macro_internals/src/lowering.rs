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
// Known issues:
//
// there are more cases where we could avoid buffers
//
// `inlawi!(x[..1])` and other guaranteed width>=1 && (shl=0 || shl=bw-1) parts
// should be infallible
//
// since we are now relying on const traits, maybe we should allow `&Bits` as
// indexes themselves
//
// (0x123u12[...]; ..64) can never succeed, there should probably be an error
//
// inlawi!(awi0[((i*8))..((i*8)+8)]).to_u8() there is an extra set of
// parenthesis that could be eliminated

use std::{fmt::Write, num::NonZeroUsize};

use awint_ext::ExtAwi;

use crate::{chars_to_string, Ast, Bind, CodeGen, ComponentType::*, EitherResult, Lower, Names};

/// Lowering of the parsed structs into Rust code.
pub fn cc_macro_code_gen<
    F0: FnMut(&str) -> String,
    F1: FnMut(ExtAwi) -> String,
    F2: FnMut(&str, Option<NonZeroUsize>, Option<&str>) -> String,
>(
    mut ast: Ast,
    mut code_gen: CodeGen<'_, F0, F1, F2>,
    names: Names,
) -> String {
    let is_returning = code_gen.return_type.is_some();

    // first check for simple infallible constant return
    if is_returning && (ast.cc.len() == 1) && (ast.cc[0].comps.len() == 1) {
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
    let mut no_vars = true;

    for concat_i in 0..ast.cc.len() {
        let concat = &ast.cc[concat_i];
        if (concat.comps.len() != 1) || (!concat.comps[0].has_full_range()) {
            need_buffer = true;
        }
        for comp_i in 0..ast.cc[concat_i].comps.len() {
            match ast.cc[concat_i].comps[comp_i].c_type {
                Unparsed => unreachable!(),
                Literal(ref awi) => {
                    ast.cc[concat_i].comps[comp_i].bind = Some(
                        l.binds
                            .insert(Bind::Literal(awi.clone()), (false, false))
                            .either(),
                    )
                }
                Variable => {
                    no_vars = false;
                    let mut chars = vec![];
                    ast.chars_assign_subtree(
                        &mut chars,
                        ast.cc[concat_i].comps[comp_i].mid_txt.unwrap(),
                    );
                    ast.cc[concat_i].comps[comp_i].bind =
                        Some(l.binds.insert(Bind::Txt(chars), (false, false)).either())
                }
                Filler => {
                    if concat_i == 0 {
                        source_has_filler = true;
                    }
                }
            }
        }
    }
    // the buffer is the same as the thing we are returning
    need_buffer |= is_returning;
    if code_gen.return_type.is_none() && no_vars {
        // for cases like `cc!(r0..r1)` or `cc!(0x123u12[r0..r1])`
        need_buffer = false;
    }

    // work backwards so we calculate only what we need
    for concat in &mut ast.cc {
        l.lower_concat(concat);
    }
    let lt_checks = l.lower_le_checks();
    let common_checks = l.lower_common_checks(&ast);

    let common_infallible = lt_checks.is_empty() && common_checks.is_empty();

    // edge case: `extawi!(zero: ..r)` is infallible with respect to range and
    // common checks but is fallible with respect to nonzero width
    let infallible = if is_returning {
        common_infallible && ast.guaranteed_nonzero_width
    } else {
        common_infallible
    };

    let construction = if need_buffer {
        let mut s = vec![];
        if let Some(init) = ast.txt_init {
            ast.chars_assign_subtree(&mut s, init);
        }
        let s = chars_to_string(&s);
        // buffer and reference to buffer
        format!(
            "let mut {}={};let {}=&mut {};\n",
            names.awi,
            (code_gen.construction_fn)(&s, ast.common_bw, Some(names.cw)),
            names.awi_ref,
            names.awi,
        )
    } else {
        String::new()
    };

    let fielding = l.lower_fielding(&ast, source_has_filler, need_buffer);

    let returning = match (is_returning, infallible) {
        (false, false) => "Some(())".to_owned(),
        (false, true) => String::new(),
        (true, false) => format!("Some({})", names.awi),
        (true, true) => names.awi.to_owned(),
    };
    let wrap_must_use = !returning.is_empty();

    // inner code consisting of the zero check, construction of returning or
    // buffers, fielding, and return values
    let mut inner0 = format!("{}{}{}", construction, fielding, returning);

    // very tricky
    if !ast.guaranteed_nonzero_width {
        if is_returning {
            if infallible {
                // do nothing
            } else {
                // avoid creating the return value and return `None` because it is a condition
                // of construction macros
                inner0 = format!("if {} != 0 {{\n{}\n}}else{{None}}", names.cw, inner0);
            }
        } else if need_buffer {
            if infallible {
                // overall infallible but we need to avoid creating the buffer
                inner0 = format!("if {} != 0 {{\n{}\n}}", names.cw, inner0);
            } else {
                // overall fallible, but this is not a construction macro, but we need to avoid
                // creating the buffer
                inner0 = format!("if {} != 0 {{\n{}\n}}else{{Some(())}}", names.cw, inner0);
            }
        } // else do nothing because there is no buffer
    }

    // concat width checks
    let inner1 = if common_checks.is_empty() {
        inner0
    } else {
        format!("if {} {{\n{}\n}}else{{None}}", common_checks, inner0)
    };

    // designate the common concatenation width
    let common_cw = if let Some(bw) = ast.common_bw {
        format!("let {}={}usize;\n", names.cw, bw)
    } else if let Some(p_cw) = l.dynamic_width {
        *l.cw.a_get_mut(p_cw) = true;
        let s = format!("let {}={}_{};\n", names.cw, names.cw, p_cw.get_raw());
        s
    } else {
        // for the case with all unbounded fillers, find the max bitwidth for the buffer
        // to use.
        let mut s = String::new();
        for concat in &ast.cc {
            if (concat.comps.len() == 1) && concat.comps[0].is_unbounded_filler() {
                continue
            }
            if !s.is_empty() {
                s += ",";
            }
            let p_cw = concat.cw.unwrap();
            *l.cw.a_get_mut(p_cw) = true;
            write!(s, "{}_{}", names.cw, p_cw.get_raw()).unwrap();
        }
        format!("let {}={}([{}]);\n", names.cw, fn_names.max_fn, s)
    };

    // width and common width calculations come after reversal checks and before
    // concat width checks
    let cws = l.lower_cws();
    let widths = l.lower_widths();
    let inner2 = format!("{}{}{}{}", widths, cws, common_cw, inner1);

    // reversal checks
    let inner3 = if lt_checks.is_empty() {
        inner2
    } else {
        format!("if {} {{\n{}\n}}else{{None}}", lt_checks, inner2)
    };

    let values = l.lower_values();
    let bindings = l.lower_bindings(code_gen.lit_construction_fn);

    let inner4 = format!("{{\n{}{}{}}}", bindings, values, inner3);
    if wrap_must_use {
        (code_gen.must_use)(&inner4)
    } else {
        inner4
    }
}
