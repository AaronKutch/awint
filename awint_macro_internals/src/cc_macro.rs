use std::{num::NonZeroUsize, str::FromStr};

use awint_ext::ExtAwi;
use proc_macro2::TokenStream;

use crate::{
    cc_macro_code_gen, error_and_help, stage1, stage2, stage3, stage4, stage5, token_stream_to_ast,
    CCMacroError, CodeGen, Names,
};

/// Input parsing and code generation function for corresponding concatenations
/// of components macros.
pub fn cc_macro<
    'a,
    F0: FnMut(&str) -> String,
    // we run into lifetime generalization issues when trying `&Bits`
    F1: FnMut(ExtAwi) -> String,
    F2: FnMut(&str, Option<NonZeroUsize>, Option<&str>) -> String,
>(
    // TODO bring out documentation once finished
    input: &str,
    // FIXME remove
    specified_init: bool,
    code_gen: CodeGen<'a, F0, F1, F2>,
    names: Names,
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

    // stage 1: basic parsing of components
    match stage1(&mut ast) {
        Ok(()) => (),
        Err(e) => return Err(e.ast_error(&ast)),
    };

    // stage 2: individual component pass
    match stage2(&mut ast) {
        Ok(()) => (),
        Err(e) => return Err(e.ast_error(&ast)),
    };

    // stage 3: individual concatenation pass
    match stage3(&mut ast) {
        Ok(()) => (),
        Err(e) => return Err(e.ast_error(&ast)),
    };

    // stage 4: cc pass accounting for macro type
    let static_width = code_gen.static_width;
    match stage4(
        &mut ast,
        specified_init,
        &code_gen.return_type,
        static_width,
    ) {
        Ok(()) => (),
        Err(e) => return Err(e.ast_error(&ast)),
    };

    // stage 5: concatenation simplification
    stage5(&mut ast);

    /*dbg!(&ast);
    #[cfg(feature = "debug")]
    triple_arena_render::render_to_svg_file(
        &ast.txt,
        false,
        std::path::PathBuf::from("./example.svg"),
    )
    .unwrap();*/

    Ok(cc_macro_code_gen(ast, specified_init, code_gen, names))
}
