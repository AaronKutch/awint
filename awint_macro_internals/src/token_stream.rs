use std::{collections::VecDeque, mem};

use proc_macro2::{Delimiter, TokenStream, TokenTree};

/// Parses `input` `TokenStream` a concatenations of components in `Vec<char>`
/// strings
pub fn token_stream_to_cc(input: TokenStream) -> Vec<Vec<Vec<char>>> {
    // The `ToString` implementation on `TokenStream`s does not recover the
    // original spacings despite that information being included in spans.
    // Frustratingly, `Span`s (as of Rust 1.61) provide no stable information about
    // their exact byte or char relative locations (despite it being in their
    // `Debug` representations, but I can't use that without a major breakage
    // risk).
    let mut concatenations = Vec::<Vec<Vec<char>>>::new();
    let mut components = Vec::<Vec<char>>::new();
    let mut string = Vec::<char>::new();
    // traverse the tree
    let mut stack: Vec<(VecDeque<TokenTree>, Delimiter)> =
        vec![(input.into_iter().collect(), Delimiter::None)];
    loop {
        let last = stack.len() - 1;
        if let Some(tt) = stack[last].0.front() {
            match tt {
                TokenTree::Group(g) => {
                    let d = g.delimiter();
                    match d {
                        Delimiter::Parenthesis => string.push('('),
                        Delimiter::Brace => string.push('{'),
                        Delimiter::Bracket => string.push('['),
                        // these are important in certain situations with `macro_rules`
                        Delimiter::None => {
                            if last != 0 {
                                string.push(' ')
                            }
                        }
                    };
                    let trees = g.stream().into_iter().collect();
                    stack[last].0.pop_front().unwrap();
                    stack.push((trees, d));
                    continue
                }
                TokenTree::Ident(i) => {
                    string.extend(i.to_string().chars());
                }
                TokenTree::Punct(p) => {
                    let p = p.as_char();
                    if (last == 0) && (p == ',') {
                        components.push(mem::take(&mut string));
                    } else if (last == 0) && (p == ';') {
                        components.push(mem::take(&mut string));
                        concatenations.push(mem::take(&mut components));
                    } else {
                        string.push(p);
                    }
                }
                TokenTree::Literal(l) => {
                    // one of the main points of going through `TokenTree` interfaces is to let the
                    // parser handle all the complexity of the possible string and char literal
                    // delimiting
                    string.extend(l.to_string().chars())
                }
            }
            stack[last].0.pop_front().unwrap();
        } else {
            match stack[last].1 {
                Delimiter::Parenthesis => string.push(')'),
                Delimiter::Brace => string.push('}'),
                Delimiter::Bracket => string.push(']'),
                Delimiter::None => {
                    if last != 0 {
                        string.push(' ')
                    }
                }
            };
            if last == 0 {
                break
            }
            stack.pop().unwrap();
        }
    }
    components.push(string);
    concatenations.push(components);
    concatenations
}
