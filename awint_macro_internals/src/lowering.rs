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
// we can implement repeating by syntax like
// `inlawi!( (expr)[..] * 4 )` `extawi!( var[..] * num_repeats )`, require
// square brackets to make sure we don't have something like a dynamic `Mul`
// implementing type
//
// should we implement it however? Most often it is used in single bit stuff, I
// can't think of occurances with more than 1 bit.

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

use awint_ext::{awint_core::OrdBits, ExtAwi};
use triple_arena::Ptr;

use crate::{chars_to_string, Ast, Bind, CodeGen, ComponentType::*, Lower, Names};

/// Lowering of the parsed structs into Rust code.
pub fn cc_macro_code_gen<
    F0: FnMut(&str) -> String,
    F1: FnMut(ExtAwi) -> String,
    F2: FnMut(ExtAwi) -> String,
    F3: FnMut(&str, Option<NonZeroUsize>, Option<&str>) -> String,
    F4: FnMut(String, Option<NonZeroUsize>, bool) -> String,
>(
    mut ast: Ast,
    mut code_gen: CodeGen<'_, F0, F1, F2, F3, F4>,
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
                            .insert((Bind::Literal(OrdBits(awi.clone())), (false, false)))
                            .0,
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
                        Some(l.binds.insert((Bind::Txt(chars), (false, false))).0)
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
    let (lt_infallible, lt_checks0, lt_checks1) = l.lower_le_checks();
    let (common_infallible, common_checks0, common_checks1) = l.lower_common_checks(&ast);

    let checks_infallible = lt_infallible && common_infallible;

    // edge case: `extawi!(zero: ..r)` is infallible with respect to range and
    // common checks but is fallible with respect to nonzero width
    let infallible = if is_returning {
        checks_infallible && ast.guaranteed_nonzero_width
    } else {
        checks_infallible
    };

    // designate the common concatenation width
    let common_cw = if let Some(bw) = ast.common_bw {
        format!("let {}={}usize;\n", names.cw, bw)
    } else if let Some(p_cw) = l.dynamic_width {
        *l.cw.get_val_mut(p_cw).unwrap() = true;
        let s = format!("let {}={}_{};\n", names.cw, names.cw, p_cw.inx());
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
            *l.cw.get_val_mut(p_cw).unwrap() = true;
            write!(s, "{}_{}", names.cw, p_cw.inx()).unwrap();
        }
        format!("let {}={}([{}]);\n", names.cw, fn_names.max_fn, s)
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
        (false, false) => String::new(),
        (false, true) => String::new(),
        (true, false) => names.awi.to_owned(),
        (true, true) => names.awi.to_owned(),
    };
    let wrap_must_use = !returning.is_empty();

    // inner code consisting of construction of Awis, fielding, and returning
    let mut inner = format!("{construction}{fielding}{returning}");

    // very tricky, there are too many corner and optimization cases to do this in a
    // more compact way
    if ast.guaranteed_nonzero_width {
        if infallible {
            // do nothing
        } else {
            inner = format!(
                "let {} = {}([{}],[{}],[{}],[{}],{},false,false);\nif {}.run_fielding() \
                 {{{}.wrap({{\n{}\n}})}} else {{{}.wrap_none()}}",
                names.res,
                fn_names.cc_checks_fn,
                lt_checks0,
                lt_checks1,
                common_checks0,
                common_checks1,
                names.cw,
                names.res,
                names.res,
                inner,
                names.res,
            )
        }
    } else if is_returning {
        if infallible {
            // do nothing
        } else {
            // avoid creating the return value and return `None` because it is a condition
            // of construction macros
            inner = format!(
                "let {} = {}([{}],[{}],[{}],[{}],{},true,false);\nif {}.run_fielding() \
                 {{{}.wrap({{\n{}\n}})}} else {{{}.wrap_none()}}",
                names.res,
                fn_names.cc_checks_fn,
                lt_checks0,
                lt_checks1,
                common_checks0,
                common_checks1,
                names.cw,
                names.res,
                names.res,
                inner,
                names.res,
            )
        }
    } else if need_buffer {
        if infallible {
            // overall infallible but we need to avoid creating the buffer
            inner = format!(
                "let {} = {}([{}],[{}],[{}],[{}],{},true,false);\nif {}.run_fielding() {{\n{}\n}}",
                names.res,
                fn_names.cc_checks_fn,
                lt_checks0,
                lt_checks1,
                common_checks0,
                common_checks1,
                names.cw,
                names.res,
                inner,
            )
        } else {
            // overall fallible, but this is not a construction macro, but we need to avoid
            // creating the buffer
            /*inner = format!(
                "if {} != 0 {{\n{}\n}}else{{core::option::Option::Some(())}}",
                names.cw, inner
            );*/
            //nonfielding_case = format!(" else {{{}.wrap_if_success()}}", names.res);
            inner = format!(
                "let {} = {}([{}],[{}],[{}],[{}],{},true,true);\nif {}.run_fielding() \
                 {{{}.wrap({{\n{}\n}})}} else {{{}.wrap_if_success()}}",
                names.res,
                fn_names.cc_checks_fn,
                lt_checks0,
                lt_checks1,
                common_checks0,
                common_checks1,
                names.cw,
                names.res,
                names.res,
                inner,
                names.res,
            )
        }
    } else {
        // no buffer
        if infallible {
            // nothing
        } else {
            inner = format!(
                "let {} = {}([{}],[{}],[{}],[{}],{},false,false);\nif {}.run_fielding() \
                 {{{}.wrap({{\n{}\n}})}} else {{{}.wrap_none()}}",
                names.res,
                fn_names.cc_checks_fn,
                lt_checks0,
                lt_checks1,
                common_checks0,
                common_checks1,
                names.cw,
                names.res,
                names.res,
                inner,
                names.res,
            )
        }
    }

    // width and common width calculations used to come after reversal checks and
    // before concat width checks, but because of `awint_dag` and because the
    // successful path is usually chosen, all calculations are done before the
    // checks
    let cws = l.lower_cws();
    let widths = l.lower_widths();
    let values = l.lower_values();
    let bindings = l.lower_bindings(code_gen.static_construction_fn);

    let inner = format!("{{\n{bindings}{values}{widths}{cws}{common_cw}{inner}}}");
    let inner = (code_gen.const_wrapper)(inner, ast.common_bw, infallible);
    if wrap_must_use {
        (code_gen.must_use)(&inner)
    } else {
        inner
    }
}
