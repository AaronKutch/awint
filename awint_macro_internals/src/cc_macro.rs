use std::str::FromStr;

use awint_ext::ExtAwi;
use proc_macro2::TokenStream;
use ComponentType::*;

use crate::{
    component::{Component, ComponentType},
    error_and_help, token_stream_to_ast, CCMacroError, Delimiter, Names, Text,
};

/// Input parsing and code generation function for corresponding concatenations
/// of components macros.
pub fn cc_macro<F: FnMut(ExtAwi) -> String>(
    // TODO bring out documentation once finished
    input: &str,
    // if initialization is specified
    specified_init: bool,
    // e.x. "ExtAwi"
    return_type: Option<&str>,
    // literal construction function. Note: should include `Bits::must_use` or similar
    lit_construction_fn: Option<F>,
    // TODO remove this
    _unstable_construction_fn: Option<&str>,
    // if a type like `InlAwi` needs statically known width
    static_width: bool,
    _names: Names,
) -> Result<String, String> {
    // we process in stages to handle more fundamental errors first, reducing bugs
    // and confusion

    // if we ever need to optimize for speed what we probably want to do is have an
    // implace implementation and rerun through this implementation when an error is
    // encountered

    // stage 0: separation into raw concatenations of components
    let mut ast = match TokenStream::from_str(input) {
        Ok(ts) => token_stream_to_ast(ts),
        // this shouldn't be possible if the input has run through a macro already, but we keep this
        // for plain `cc_macro` uses
        Err(e) => {
            // lex error displays as "cannot parse string into token stream" which is not
            // good enough, try to determine if there is a mismatched delimiter
            let mut parenthesis = (0, 0);
            let mut brackets = (0, 0);
            let mut braces = (0, 0);
            for c in input.chars() {
                match c {
                    '(' => parenthesis.0 += 1,
                    ')' => parenthesis.1 += 1,
                    '[' => brackets.0 += 1,
                    ']' => brackets.1 += 1,
                    '{' => braces.0 += 1,
                    '}' => braces.1 += 1,
                    _ => (),
                }
            }
            let note = if parenthesis.0 != parenthesis.1 {
                format!(
                    "\nnote: there are {} '(' chars and {} ')' chars",
                    parenthesis.0, parenthesis.1
                )
            } else if brackets.0 != brackets.1 {
                format!(
                    "\nnote: there are {} '[' chars and {} ']' chars",
                    brackets.0, brackets.1
                )
            } else if braces.0 != braces.1 {
                format!(
                    "\nnote: there are {} '{{' chars and {} '}}' chars",
                    braces.0, braces.1
                )
            } else {
                "".to_owned()
            };
            return Err(error_and_help(&format!("input failed to tokenize: {}{}", e, note),
                "for further information see the library documentation of `awint_macros` \
                https://docs.rs/awint_macros/"))
        }
    };

    // trailing punctuation handling and reversing concatenations
    let mut trailing_semicolon = false;
    let mut trailing_commas = vec![];
    let raw_cc_len = ast.cc.len();
    for (concat_i, concat) in ast.cc.iter().enumerate() {
        let concat_len = concat.comps.len();
        if (concat.comps.len() == 1) && ast.txt[concat.comps[0].txt].is_empty() {
            if (concat_i + 1) != raw_cc_len {
                return Err(CCMacroError::new(
                    "Empty concatenation, at most a single trailing semicolon is allowed"
                        .to_owned(),
                    concat.txt,
                )
                .ast_error(&ast))
            }
            trailing_semicolon = true;
        }
        for (comp_i, comp) in concat.comps.iter().enumerate() {
            if ast.txt[comp.txt].is_empty() {
                if (comp_i + 1) != concat_len {
                    return Err(CCMacroError::new(
                        "Empty component before end of concatenation, at most a single trailing \
                         comma is allowed"
                            .to_owned(),
                        comp.txt,
                    )
                    .ast_error(&ast))
                }
                trailing_commas.push(concat_i);
            }
        }
    }
    for comma_i in trailing_commas {
        ast.cc[comma_i].comps.pop().unwrap();
    }
    if trailing_semicolon {
        ast.cc.pop().unwrap();
    }
    if ast.cc.is_empty() {
        return Err(error_and_help("empty input", "for further information see the \
        library documentation of `awint_macros` https://docs.rs/awint_macros/"))
    }
    // Components are written like `component N, component N - 1`, I ultimately made
    // this decision so that literals next to each other would concatenate
    // visually
    for concat in &mut ast.cc {
        concat.comps.reverse();
    }

    panic!();
    /*dbg!(&ast);
    #[cfg(feature = "debug")]
    triple_arena_render::render_to_svg_file(
        &ast.txt,
        false,
        std::path::PathBuf::from("./example.svg"),
    )
    .unwrap();*/
    /*

    // stage 1: basic parsing of components
    let mut cc = match parse_cc(&raw_cc) {
        Ok(cc) => cc,
        Err(e) => return Err(e.raw_cc_error(&raw_cc)),
    };

    // stage 2: individual component pass
    match stage2(&mut cc) {
        Ok(()) => (),
        Err(e) => return Err(e.raw_cc_error(&raw_cc)),
    };

    // stage 3: individual concatenation pass
    match stage3(&mut cc) {
        Ok(()) => (),
        Err(e) => return Err(e.raw_cc_error(&raw_cc)),
    };

    // stage 4: cc pass accounting for macro type
    match stage4(&mut cc, specified_init, return_type, static_width) {
        Ok(()) => (),
        Err(e) => return Err(e.raw_cc_error(&raw_cc)),
    };

    // stage 5: concatenation simplification
    stage5(&mut cc);

    // code gen

    // first check for simple infallible constant return
    if return_type.is_some() && (cc.len() == 1) && (cc[0].comps.len() == 1) {
        let comp = &cc[0].comps[0];
        if let Literal(ref lit) = comp.c_type {
            // constants have been normalized and combined by now
            if comp.range.static_range().is_some() {
                return Ok(lit_construction_fn.unwrap()(ExtAwi::from_bits(lit)))
            }
        }
    }
    */

    Ok("todo!()".to_owned())
}
