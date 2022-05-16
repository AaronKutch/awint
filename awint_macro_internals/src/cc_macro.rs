use std::str::FromStr;

use proc_macro2::TokenStream;

use crate::{parse_cc, token_stream_to_raw_cc};

/// Input parsing and code generation function for corresponding concatenations
/// of components macros.
pub fn cc_macro(input: &str) -> Result<String, String> {
    // we process in stages to handle more fundamental errors first, reducing user
    // confusion stage 0: separation into raw concatenations of components
    let raw_cc = match TokenStream::from_str(input) {
        Ok(ts) => token_stream_to_raw_cc(ts),
        // this shouldn't be possible if the input has run through a macro already, but we keep this
        // for plain `cc_macro` uses
        Err(e) => return Err(format!("input failed to tokenize:\n{}", e)),
    };
    // stage 1: basic parsing of components
    let cc0 = match parse_cc(&raw_cc) {
        Ok(cc) => cc,
        Err(e) => return Err(e.raw_cc_error(&raw_cc)),
    };
    // stage 2: individual component pass

    Ok("todo!()".to_owned())
}
