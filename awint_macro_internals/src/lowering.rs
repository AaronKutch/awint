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

use awint_ext::ExtAwi;
use triple_arena::{ptr_trait_struct_with_gen, Ptr};

use crate::{Ast, BiMap, ComponentType::*, Names};

ptr_trait_struct_with_gen!(PBind);

/// Note: `Bits::must_use` or analogous should wrap some of these
///
/// `static_width` - if the type needs a statically known width
/// `return_type` - if the bits need to be returned
/// `lit_construction_fn` - construction function for known literals
pub struct CodeGen<'a, F0: FnMut(ExtAwi) -> String> {
    pub static_width: bool,
    pub return_type: Option<&'a str>,
    pub lit_construction_fn: Option<F0>,
}

/// Lowering of the parsed structs into Rust code.
pub fn cc_macro_code_gen<'a, F0: FnMut(ExtAwi) -> String>(
    ast: &Ast,
    specified_init: bool,
    code_gen: CodeGen<'a, F0>,
    names: Names,
) -> String {
    // first check for simple infallible constant return
    if code_gen.return_type.is_some() && (ast.cc.len() == 1) && (ast.cc[0].comps.len() == 1) {
        let comp = &ast.cc[0].comps[0];
        if let Literal(ref lit) = comp.c_type {
            // constants have been normalized and combined by now
            if comp.range.static_range().is_some() {
                return code_gen.lit_construction_fn.unwrap()(ExtAwi::from_bits(lit))
            }
        }
    }

    let mut code = String::new();

    let cc = &ast.cc;

    let mut bindings: BiMap<PBind, Vec<char>, ()> = BiMap::new();
    let mut bindings_ptrs: Vec<Ptr<PBind>> = vec![];
    // get unique variables used in sinks
    for concat in &cc[1..] {
        for comp in &concat.comps {
            if let Variable(ref s) = comp.c_type {
                bindings
                    .insert_with(s.clone(), |p| {
                        bindings_ptrs.push(p);
                    })
                    .unwrap();
            }
        }
    }

    let mut need_buffer = false;

    for comp in &cc[0].comps {
        if let Variable(ref s) = comp.c_type {
            match bindings.insert(s.clone(), ()) {
                Ok(p) => bindings_ptrs.push(p),
                // the same variable is in the source concatenation and a sink concatenation
                Err(_) => need_buffer = true,
            }
        }
    }

    if let Some(return_type) = code_gen.return_type {
        //
    }

    // check for simplest copy `a[..]; b[..]; c[..]; ...` cases
    /*let mut all_copy_assign = true;
    for concat in cc {
        if (concat.comps.len() != 1) || !concat.comps[0].has_full_range() {
            all_copy_assign = false;
            break
        }
    }*/

    // true if the input is of the form
    // `constant; a[..]; b[..]; c[..]; ...` or
    // `single full range var; a[..]; b[..]; c[..]; ...`
    // let no_buffer = return_type.is_none()
    //     && all_copy_assign
    //     && (cc[0].comps.len() == 1)
    //     && cc[0].comps[0].has_full_range()
    //     && !matches!(cc[0].comps[0].c_type, Filler);
    /*let mut constructing = if return_type.is_some() {
        if static_width {
            format!(
                "let mut {} = {}::{}();\n",
                AWI,
                unstable_native_inlawi_ty(total_bw.unwrap().get() as u128),
                construct_fn
            )
        } else {
            // even if the bitwidth is known statically, we return `ExtAwi` from `extawi!`
            format!(
                "let mut {} = ExtAwi::panicking_{}({});\n",
                AWI, construct_fn, BW
            )
        }
    } else if no_buffer {
        String::new()
    } else {
        // still need a temporary, `AWI` is not actually returned
        if let Some(bw) = total_bw {
            format!(
                "let mut {} = {}::{}();\n",
                AWI,
                unstable_native_inlawi_ty(bw.get() as u128),
                construct_fn
            )
        } else {
            format!(
                "let mut {} = ExtAwi::panicking_{}({});\n",
                AWI, construct_fn, BW
            )
        }
    };
    if !no_buffer {
        constructing += &format!("let {}: &mut Bits = {}.const_as_mut();\n", AWI_REF, AWI);
    }

    let mut filler_in_source = false;
    for comp in &concats[0].concatenation {
        if let Filler = comp.component_type {
            filler_in_source = true;
            break
        }
    }

    let mut fielding = String::new();

    if filler_in_source && !specified_initialization {
        // note: in all cases that reach here the source must be `AWI_REF`
        for j0 in 1..concats.len() {
            let concat = &concats[j0];
            let use_copy_assign =
                concat.concatenation.len() == 1 && concat.concatenation[0].has_full_range();
            // see notes at top of file
            // sink -> buffer
            if use_copy_assign {
                // use copy assign
                let sink_comp = &concat.concatenation[0];
                if let Some(sink_name) = lowered_name(Some(&l.literals), sink_comp) {
                    fielding += &format!(
                        "{}.const_as_mut().copy_assign({}_{}).unwrap();\n",
                        AWI_REF.to_owned(),
                        REF,
                        l.refs.get_id(&sink_name)
                    );
                    l.used_ref_refs.insert(sink_name);
                } // else it is a no-op filler
            } else {
                fielding += &l.lower_fielding_to_awi(concat);
            }
            // no general possibilty for copy assigning, because there is a sink
            // source -> buffer
            fielding += &l.lower_fielding_to_awi(&concats[0]);
            // buffer -> sink
            if use_copy_assign {
                // use copy assign
                let sink_comp = &concat.concatenation[0];
                if let Some(sink_name) = lowered_name(Some(&l.literals), sink_comp) {
                    fielding += &format!(
                        "{}_{}.const_as_mut().copy_assign({}).unwrap();\n",
                        REF,
                        l.refs.get_id(&sink_name),
                        AWI_REF.to_owned()
                    );
                    l.used_mut_refs.insert(sink_name);
                }
            } else {
                fielding += &l.lower_fielding_from_awi(concat);
            }
        }
    } else if no_buffer {
        let name = lowered_name(Some(&l.literals), &concats[0].concatenation[0]).unwrap();
        let source_name = format!("{}_{}", REF, l.refs.get_id(&name));
        l.used_ref_refs.insert(name);
        // simplest copy assigning
        for concat in &concats[1..] {
            let sink_comp = &concat.concatenation[0];
            if let Some(sink_name) = lowered_name(Some(&l.literals), sink_comp) {
                fielding += &format!(
                    "{}_{}.const_as_mut().copy_assign({}).unwrap();\n",
                    REF,
                    l.refs.get_id(&sink_name),
                    source_name
                );
                l.used_mut_refs.insert(sink_name);
            }
        }
    } else {
        // source -> buffer once
        if concats[0].concatenation.len() == 1 && concats[0].concatenation[0].has_full_range() {
            let sink_comp = &concats[0].concatenation[0];
            if let Some(sink_name) = lowered_name(Some(&l.literals), sink_comp) {
                fielding += &format!(
                    "{}.const_as_mut().copy_assign({}_{}).unwrap();\n",
                    AWI_REF.to_owned(),
                    REF,
                    l.refs.get_id(&sink_name)
                );
                l.used_ref_refs.insert(sink_name);
            }
        } else {
            fielding += &l.lower_fielding_to_awi(&concats[0]);
        }
        // buffer -> sinks
        for concat in &concats[1..] {
            if concat.concatenation.len() == 1 && concat.concatenation[0].has_full_range() {
                let sink_comp = &concat.concatenation[0];
                if let Some(sink_name) = lowered_name(Some(&l.literals), sink_comp) {
                    fielding += &format!(
                        "{}_{}.const_as_mut().copy_assign({}).unwrap();\n",
                        REF,
                        l.refs.get_id(&sink_name),
                        AWI_REF.to_owned()
                    );
                    l.used_mut_refs.insert(sink_name);
                }
            } else {
                fielding += &l.lower_fielding_from_awi(concat);
            }
        }
    }

    // bindings that need to be mutable
    let mut mutable_bindings = HashSet::<u64>::new();

    // lower all used references by assigning them to `let` bindings
    let mut referencing = String::new();
    for reference in &l.used_mut_refs {
        // mutable bindings supersede immutable ones
        l.used_ref_refs.remove(reference);
        let id = l.bindings.get_id(reference);
        referencing += &format!(
            "let {}_{}: &mut Bits = {}_{}.const_as_mut();\n",
            REF,
            l.refs.get_and_set_used(reference).0,
            BINDING,
            id
        );
        mutable_bindings.insert(id);
    }
    for reference in &l.used_ref_refs {
        referencing += &format!(
            "let {}_{}: &Bits = {}_{}.const_as_ref();\n",
            REF,
            l.refs.get_and_set_used(reference).0,
            BINDING,
            l.bindings.get_id(reference)
        );
    }
    for (ptr, (_, _, used)) in l.refs.arena() {
        if *used {
            l.bindings.set_used(l.ref_to_binding[&ptr]);
        }
    }

    // Lower all used widths by calculating them. This uses more values, which is
    // why this must run first. Overflow is not possible because of component
    // checks.
    let mut s_widths = String::new();
    for (ptr, (id, width, used)) in l.widths.arena() {
        if *used {
            match width {
                Width::Single(ref s) => {
                    s_widths +=
                        &format!("let {}_{}: usize = {};\n", WIDTH, id, l.string_to_value[s]);
                }
                Width::Range(ref s0, ref s1) => {
                    s_widths += &format!(
                        "let {}_{}: usize = {}.wrapping_sub({});\n",
                        WIDTH, id, l.string_to_value[s1], l.string_to_value[s0],
                    );
                }
            }
            l.values.set_used(l.width_to_value[&ptr]);
        }
    }

    // lower all used values
    let mut s_values = String::new();
    for (ptr, (id, val, used)) in l.values.arena() {
        if *used {
            s_values += &format!("let {}_{}: usize = {};\n", VALUE, id, val);
            l.bindings.set_used(l.value_to_binding[&ptr]);
        }
    }

    // lower all used bindings by assigning them to `let` bindings
    let mut s_bindings = String::new();
    for (id, val, used) in l.bindings.arena().vals() {
        if *used {
            if mutable_bindings.contains(id) {
                s_bindings += &format!("let mut {}_{} = {};\n", BINDING, id, val);
            } else {
                s_bindings += &format!("let {}_{} = {};\n", BINDING, id, val);
            }
        }
    }

    let infallible = l.concat_lt_partials.is_empty()
        && l.concat_ne_partials.is_empty()
        && l.comp_check_partials.is_empty();

    let returning = match (return_source, infallible) {
        (false, false) => "Some(())".to_owned(),
        (false, true) => String::new(),
        (true, false) => format!("Some({})", AWI),
        (true, true) => AWI.to_owned(),
    };

    // construct the output code by starting with the innermost scope
    let mut output = if !return_source && (concats.len() < 2) {
        // for cases where nothing is copied or constructed
        format!("\n{}\n", returning)
    } else {
        format!(
            "\n{}\n{}\n{}\n{}\n",
            referencing, constructing, fielding, returning
        )
    };

    if !infallible {
        if return_source {
            output = format!("if {} != 0 {{\n{}\n}} else {{None}}", BW, output);
        } else {
            // Non-construction macros can have a zero concatenation bitwidth, but we have
            // to avoid creating the buffer.
            output = format!("if {} != 0 {{\n{}\n}} else {{Some(())}}", BW, output);
        }
    }

    match (
        l.concat_ne_partials.is_empty(),
        l.concat_lt_partials.is_empty(),
    ) {
        (true, true) => (),
        (true, false) => {
            output = format!(
                "if Bits::unstable_common_lt_checks({}, {}).is_some() {{\n{}\n}} else {{None}}",
                BW,
                array_partials(l.concat_lt_partials),
                output
            )
        }
        (false, true) => {
            output = format!(
                "if Bits::unstable_common_ne_checks({}, {}).is_some() {{\n{}\n}} else {{None}}",
                BW,
                array_partials(l.concat_ne_partials),
                output
            )
        }
        (false, false) => {
            output = format!(
                "if Bits::unstable_common_ne_checks({}, {}).is_some()\n&& \
                 Bits::unstable_common_lt_checks({}, {}).is_some() {{\n{}\n}} else {{None}}",
                BW,
                array_partials(l.concat_ne_partials),
                BW,
                array_partials(l.concat_lt_partials),
                output
            )
        }
    }
    output = format!(
        "{}\n{}\n{}\n{}",
        s_widths, l.s_bitwidths, l.common_bw, output
    );
    if !l.lt_checks.arena().is_empty() {
        output = format!(
            "if Bits::unstable_lt_checks({}).is_some() {{\n{}\n}} else {{None}}",
            array_partials(l.comp_check_partials),
            output
        );
    }
    output = format!(
        "{{\n{}\n{}\n{}\n{}\n}}",
        l.s_literals, s_bindings, s_values, output
    );*/
    code
}
