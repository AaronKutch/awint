use std::str::FromStr;

use proc_macro2::TokenStream;

use crate::{
    error_and_help, parse_cc, stage2, stage3, stage4, stage5, token_stream_to_raw_cc, CCMacroError,
    Names,
};

/// Input parsing and code generation function for corresponding concatenations
/// of components macros.
pub fn cc_macro(
    input: &str,
    // if initialization is specified
    specified_init: bool,
    // e.x. "ExtAwi"
    construction_type: Option<&str>,
    // TODO `construction_fn` will be from colon separated part
    // e.x. "umax" or "custom_fn_from_trait"
    _construction_fn: Option<&str>,
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
    let mut raw_cc = match TokenStream::from_str(input) {
        Ok(ts) => token_stream_to_raw_cc(ts),
        // this shouldn't be possible if the input has run through a macro already, but we keep this
        // for plain `cc_macro` uses
        Err(e) => {
            // lex error displays as "cannot parse string into token stream" which is not
            // good enough, try to determine if there is a mismatched delimiter TODO
            return Err(error_and_help(&format!("input failed to tokenize: {}", e),
                "for further information see the library documentation of `awint_macros` \
                https://docs.rs/awint_macros/"))
        }
    };
    let empty: Vec<Vec<char>> = vec![vec![]];
    if raw_cc[0] == empty {
        return Err(error_and_help("empty input", "for further information see the \
        library documentation of `awint_macros` https://docs.rs/awint_macros/"))
    }
    // trailing punctuation handling and reversing concatenations
    let mut trailing_semicolon = false;
    let mut trailing_commas = vec![];
    let mut error = (None, None);
    let raw_cc_len = raw_cc.len();
    for (concat_i, concat) in raw_cc.iter().enumerate() {
        let concat_len = concat.len();
        if *concat == empty {
            if (concat_i + 1) != raw_cc_len {
                error = (Some(concat_i), None);
            }
            trailing_semicolon = true;
        }
        for (comp_i, comp) in concat.iter().enumerate() {
            if comp.is_empty() {
                if (comp_i + 1) != concat_len {
                    error = (Some(concat_i), Some(concat_len - 1 - comp_i));
                }
                trailing_commas.push(concat_i);
            }
        }
    }
    // Components are written like `component N, component N - 1`, this makes logic
    // easier. I ultimately made this decision so that literals next to each
    // other would concatenate visually
    for concat in &mut raw_cc {
        concat.reverse();
    }
    // do this after reversal and before removal so that the errors display
    // correctly
    match error {
        (Some(concat_i), None) => {
            return Err(CCMacroError {
                concat_i: Some(concat_i),
                error: "Empty concatenation, at most a single trailing semicolon is allowed"
                    .to_owned(),
                ..Default::default()
            }
            .raw_cc_error(&raw_cc))
        }
        (Some(concat_i), Some(comp_i)) => {
            return Err(CCMacroError {
                concat_i: Some(concat_i),
                comp_i: Some(comp_i),
                error: "Empty component at the end, at most a single trailing comma is allowed"
                    .to_owned(),
                ..Default::default()
            }
            .raw_cc_error(&raw_cc))
        }
        _ => {}
    }
    while let Some(concat_i) = trailing_commas.pop() {
        raw_cc[concat_i].remove(0);
    }
    if trailing_semicolon {
        raw_cc.pop().unwrap();
    }

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
    match stage4(&mut cc, specified_init, construction_type, static_width) {
        Ok(()) => (),
        Err(e) => return Err(e.raw_cc_error(&raw_cc)),
    };

    // stage 5: concatenation simplification
    stage5(&mut cc);

    Ok("todo!()".to_owned())
}
