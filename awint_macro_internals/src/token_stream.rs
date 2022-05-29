use std::{collections::VecDeque, mem};

use proc_macro2::{TokenStream, TokenTree};
use triple_arena::{Arena, Ptr};

use crate::{Ast, Component, Concatenation, Text, Usbr};

/// Parses `input` `TokenStream` into "raw" concatenations of components in
/// `Vec<char>` strings
pub fn token_stream_to_ast(input: TokenStream) -> Ast {
    // The `ToString` implementation on `TokenStream`s does not recover the
    // original spacings despite that information being included in spans.
    // Frustratingly, `Span`s (as of Rust 1.61) provide no stable information about
    // their exact byte or char relative locations (despite it being in their
    // `Debug` representations, but I can't use that without a major breakage
    // risk).

    let mut ast = Ast {
        txt: Arena::new(),
        txt_root: Ptr::invalid(),
        cc: vec![],
        common_bw: None,
        deterministic_width: false,
    };
    let mut s = Vec::<char>::new();
    // traverse the tree
    let mut stack: Vec<(VecDeque<TokenTree>, proc_macro2::Delimiter)> =
        vec![(input.into_iter().collect(), proc_macro2::Delimiter::None)];
    // converting into a new tree, these first three levels always have the same
    // three delimiters
    let mut ast_stack: Vec<(Vec<Text>, crate::Delimiter)> = vec![
        (vec![], crate::Delimiter::None),
        (vec![], crate::Delimiter::Concatenation),
        (vec![], crate::Delimiter::Component),
    ];
    loop {
        let last = stack.len() - 1;
        let ast_last = ast_stack.len() - 1;
        if let Some(tt) = stack[last].0.front() {
            match tt {
                TokenTree::Group(g) => {
                    let d = g.delimiter();
                    match d {
                        proc_macro2::Delimiter::Parenthesis => {
                            ast_stack.push((vec![], crate::Delimiter::Parenthesis))
                        }
                        proc_macro2::Delimiter::Brace => {
                            ast_stack.push((vec![], crate::Delimiter::Brace))
                        }
                        proc_macro2::Delimiter::Bracket => {
                            ast_stack.push((vec![], crate::Delimiter::Bracket))
                        }
                        // these are important in certain situations with `macro_rules`
                        proc_macro2::Delimiter::None => {
                            ast_stack.push((vec![], crate::Delimiter::Space))
                        }
                    };
                    let trees = g.stream().into_iter().collect();
                    stack[last].0.pop_front().unwrap();
                    stack.push((trees, d));
                    continue
                }
                TokenTree::Ident(i) => {
                    s.extend(i.to_string().chars());
                    let mut another_ident = false;
                    if stack[last].0.len() > 1 {
                        if let TokenTree::Ident(_) = stack[last].0[1] {
                            // Special case to prevent things like "as usize" from getting squashed
                            // together as "asusize".
                            s.push(' ');
                            another_ident = true;
                        }
                    }
                    if !another_ident {
                        ast_stack[ast_last].0.push(Text::Chars(mem::take(&mut s)));
                    }
                }
                TokenTree::Punct(p) => {
                    let p = p.as_char();
                    if (last == 0) && (p == ',') {
                        assert_eq!(ast_last, 2);
                        let comp = ast.txt.insert(ast_stack.pop().unwrap().0);
                        ast_stack
                            .last_mut()
                            .unwrap()
                            .0
                            .push(Text::Group(crate::Delimiter::Component, comp));
                        ast_stack.push((vec![], crate::Delimiter::Component));
                    } else if (last == 0) && (p == ';') {
                        assert_eq!(ast_last, 2);
                        let comp = ast.txt.insert(ast_stack.pop().unwrap().0);
                        ast_stack
                            .last_mut()
                            .unwrap()
                            .0
                            .push(Text::Group(crate::Delimiter::Component, comp));
                        let concat = ast.txt.insert(ast_stack.pop().unwrap().0);
                        ast_stack
                            .last_mut()
                            .unwrap()
                            .0
                            .push(Text::Group(crate::Delimiter::Concatenation, concat));
                        ast_stack.push((vec![], crate::Delimiter::Component));
                        ast_stack.push((vec![], crate::Delimiter::Concatenation));
                    } else {
                        s.push(p);
                        ast_stack[ast_last].0.push(Text::Chars(mem::take(&mut s)));
                    }
                }
                TokenTree::Literal(l) => {
                    // One of the main points of going through `TokenTree` interfaces is to let the
                    // parser handle all the complexity of the possible string and char literal
                    // delimiting. Note: do not add spaces like for identifiers, there are various
                    // combinations that would break input.
                    s.extend(l.to_string().chars());
                    ast_stack[ast_last].0.push(Text::Chars(mem::take(&mut s)));
                }
            }
            stack[last].0.pop_front().unwrap();
        } else {
            if last == 0 {
                assert_eq!(ast_stack.len(), 3);
                let comp = ast.txt.insert(ast_stack.pop().unwrap().0);
                ast_stack
                    .last_mut()
                    .unwrap()
                    .0
                    .push(Text::Group(crate::Delimiter::Component, comp));
                let concat = ast.txt.insert(ast_stack.pop().unwrap().0);
                ast_stack
                    .last_mut()
                    .unwrap()
                    .0
                    .push(Text::Group(crate::Delimiter::Concatenation, concat));
                let root = ast.txt.insert(ast_stack.pop().unwrap().0);
                ast.txt_root = root;
                break
            }
            let (group, delimiter) = ast_stack.pop().unwrap();
            let txt = ast.txt.insert(group);
            ast_stack
                .last_mut()
                .unwrap()
                .0
                .push(Text::Group(delimiter, txt));
            stack.pop().unwrap();
        }
    }
    // iteration over the ast is cumbersome, the cc level at least is linear and
    // should be in some structs
    let root = ast.txt_root;
    let cc_len = ast.txt[root].len();
    for concat_i in 0..cc_len {
        match ast.txt[root][concat_i] {
            Text::Group(crate::Delimiter::Concatenation, p_concat) => {
                let mut concat = Concatenation {
                    txt: p_concat,
                    comps: vec![],
                    total_bw: None,
                    filler_alignment: crate::FillerAlign::None,
                    deterministic_width: false,
                };
                let c_len = ast.txt[p_concat].len();
                for comp_i in 0..c_len {
                    match ast.txt[p_concat][comp_i] {
                        Text::Group(crate::Delimiter::Component, p_comp) => {
                            concat.comps.push(Component {
                                txt: p_comp,
                                mid_txt: None,
                                range_txt: None,
                                c_type: crate::component::ComponentType::Unparsed,
                                range: Usbr::unbounded(),
                                binding: None,
                            });
                        }
                        _ => unreachable!(),
                    }
                }
                ast.cc.push(concat);
            }
            _ => unreachable!(),
        }
    }
    ast
}
