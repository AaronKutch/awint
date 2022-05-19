use std::{collections::VecDeque, mem, str::FromStr};

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use triple_arena::{Arena, PtrTrait};

use crate::{
    chars_to_string,
    ranges::{Usb, Usbr},
    token_tree::Ast,
    Text,
};

// TODO we could probably keep around the token tree in some custom structure
// rather than call `TokenStream::from_str` repeatedly on the same chars

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
        text: Arena::new(),
        comps: Arena::new(),
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
                            // special case to prevent things like "as usize" from getting squashed
                            // together as "asusize"
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
                        let comp = ast.text.insert(ast_stack.pop().unwrap().0);
                        ast_stack
                            .last_mut()
                            .unwrap()
                            .0
                            .push(Text::Group(crate::Delimiter::Component, comp));
                        ast_stack.push((vec![], crate::Delimiter::Component));
                    } else if (last == 0) && (p == ';') {
                        assert_eq!(ast_last, 2);
                        let comp = ast.text.insert(ast_stack.pop().unwrap().0);
                        ast_stack
                            .last_mut()
                            .unwrap()
                            .0
                            .push(Text::Group(crate::Delimiter::Component, comp));
                        let concat = ast.text.insert(ast_stack.pop().unwrap().0);
                        ast_stack
                            .last_mut()
                            .unwrap()
                            .0
                            .push(Text::Group(crate::Delimiter::Concatenation, concat));
                        ast_stack.push((vec![], crate::Delimiter::Component));
                        ast_stack.push((vec![], crate::Delimiter::Concatenation));
                    } else {
                        ast_stack[ast_last].0.push(Text::Chars(mem::take(&mut s)));
                        s.push(p);
                    }
                }
                TokenTree::Literal(l) => {
                    // one of the main points of going through `TokenTree` interfaces is to let the
                    // parser handle all the complexity of the possible string and char literal
                    // delimiting
                    s.extend(l.to_string().chars());
                    ast_stack[ast_last].0.push(Text::Chars(mem::take(&mut s)));
                }
            }
            stack[last].0.pop_front().unwrap();
        } else {
            let (group, delimiter) = ast_stack.pop().unwrap();
            let text = ast.text.insert(group);
            ast_stack
                .last_mut()
                .unwrap()
                .0
                .push(Text::Group(delimiter, text));
            if last == 0 {
                break
            }
            stack.pop().unwrap();
        }
    }
    ast
}

/*
/// Parses a raw component using token trees. Looks for the existence of a top
/// level "[]" delimited group and uses the last one as a bit range. If
/// `check_for_init` is set also breaks up at top level ":" and returns left
/// part in the first part of the tuple. Note: `Component::simplify` needs to be
/// run.
pub fn parse_component(
    input: &[char],
    check_for_init: bool,
) -> Result<(Option<Vec<char>>, Component), String> {
    let input = if let Ok(ts) = TokenStream::from_str(&chars_to_string(input)) {
        ts
    } else {
        // something is certainly wrong that will continue to be wrong in the code gen
        return Err("failed to tokenize".to_owned())
    };
    // traverse the tree
    let mut stack: Vec<(VecDeque<TokenTree>, Delimiter)> =
        vec![(input.into_iter().collect(), Delimiter::None)];
    let mut string: Vec<char> = vec![];
    let mut initialization: Option<Vec<char>> = None;
    let mut component_middle: Option<Vec<char>> = None;
    let mut component_range: Option<Vec<char>> = None;
    loop {
        let last = stack.len() - 1;
        let current_lvl_len = stack[last].0.len();
        if let Some(tt) = stack[last].0.front() {
            match tt {
                TokenTree::Group(g) => {
                    let d = g.delimiter();
                    match d {
                        Delimiter::Parenthesis => string.push('('),
                        Delimiter::Brace => string.push('{'),
                        Delimiter::Bracket => {
                            if (last == 0) && (current_lvl_len == 1) {
                                // top level and last group
                                assert!(component_middle.is_none());
                                component_middle = Some(mem::take(&mut string));
                            } else {
                                string.push('[');
                            }
                        }
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
                    // the spacing check is to exclude semicolons in "::" separators
                    if check_for_init
                        && (last == 0)
                        && initialization.is_none()
                        && (p.as_char() == ':')
                        && matches!(p.spacing(), Spacing::Alone)
                    {
                        initialization = Some(mem::take(&mut string));
                    } else {
                        string.push(p.as_char());
                    }
                }
                TokenTree::Literal(l) => string.extend(l.to_string().chars()),
            }
            stack[last].0.pop_front().unwrap();
        } else {
            match stack[last].1 {
                Delimiter::Parenthesis => string.push(')'),
                Delimiter::Brace => string.push('}'),
                Delimiter::Bracket => {
                    if (last == 1) && stack[0].0.is_empty() {
                        component_range = Some(mem::take(&mut string));
                    } else {
                        string.push(']')
                    }
                }
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
    if component_range.is_some() {
        assert!(string.is_empty());
    } else {
        component_middle = Some(string);
    }
    match (component_middle, component_range) {
        (None, None) => {
            if initialization.is_some() {
                // specialize this case to prevent confusion
                Err("specified initialization is followed by empty component".to_owned())
            } else {
                Err("empty component".to_owned())
            }
        }
        (Some(middle), Some(range)) => {
            if range.is_empty() {
                Err("has an empty index".to_owned())
            } else {
                match parse_range(&range, true) {
                    Ok(range) => Ok((
                        initialization,
                        Component::new(ComponentType::Variable(middle), range),
                    )),
                    Err(e) => Err(format!(
                        r#"could not parse range "{}": {}"#,
                        chars_to_string(&range),
                        e
                    )),
                }
            }
        }
        (Some(middle), None) => {
            // possibly a filler, check if is a range
            if let Ok(range) = parse_range(&middle, false) {
                Ok((initialization, Component::new(ComponentType::Filler, range)))
            } else {
                Ok((
                    initialization,
                    Component::new(ComponentType::Variable(middle), Usbr::unbounded()),
                ))
            }
        }
        _ => unreachable!(),
    }
}*/

/// Tries to parse raw `input` as a range. Looks for the existence of top
/// level ".." or "..=" punctuation. If `allow_single_bit_range` is set, will
/// return a single bit range if ".." or "..=" does not exist.
pub fn parse_range(input: &[char], allow_single_bit_range: bool) -> Result<Usbr, String> {
    let input = if let Ok(ts) = TokenStream::from_str(&chars_to_string(input)) {
        ts
    } else {
        return Err("failed to tokenize range".to_owned())
    };
    // traverse the tree
    let mut stack: Vec<(VecDeque<TokenTree>, Delimiter)> =
        vec![(input.into_iter().collect(), Delimiter::None)];
    let mut string: Vec<char> = vec![];
    let mut range = None;
    // this is toggled for char lookahead logic
    let mut is_range = false;
    let mut inclusive = false;
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
                    let p0 = p.as_char();
                    let len = stack[0].0.len();
                    if (last == 0) && (p0 == '.') && (len >= 2) {
                        if let TokenTree::Punct(ref p1) = stack[0].0[1] {
                            if p1.as_char() == '.' {
                                if range.is_some() {
                                    return Err("encountered two top level \"..\" strings in same \
                                                range"
                                        .to_owned())
                                }
                                is_range = true;
                                if len >= 3 {
                                    if let TokenTree::Punct(ref p2) = stack[0].0[2] {
                                        if p2.as_char() == '=' {
                                            // inclusive range
                                            inclusive = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if is_range {
                        range = Some(Usbr {
                            start: Some(Usb {
                                s: mem::take(&mut string),
                                x: 0,
                            }),
                            end: None,
                        });
                    } else {
                        string.push(p0);
                    }
                }
                TokenTree::Literal(l) => string.extend(l.to_string().chars()),
            }
            stack[last].0.pop_front().unwrap();
            if is_range {
                stack[last].0.pop_front().unwrap();
                if inclusive {
                    stack[last].0.pop_front().unwrap();
                }
                is_range = false;
            }
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
    if let Some(mut range) = range {
        if inclusive {
            range.end = Some(Usb {
                s: mem::take(&mut string),
                x: 1,
            });
        } else {
            range.end = Some(Usb {
                s: mem::take(&mut string),
                x: 0,
            });
        }
        Ok(range)
    } else {
        // single bit range
        if allow_single_bit_range {
            Ok(Usbr::single_bit(&string))
        } else {
            Err(r#"did not find ".." or "..=""#.to_owned())
        }
    }
}

/// In ranges we commonly see stuff like `(x + y)` or `(x - y)` with one of them
/// being a contant we can parse, which passes upward the `Usb` and `Usbr` chain
/// to get calculated into a static width. Returns `Ok(None)` if no
/// optimizations happened
pub fn usb_common_case(original: &Usb) -> Result<Option<Usb>, String> {
    let input = if let Ok(ts) = TokenStream::from_str(&chars_to_string(&original.s)) {
        ts
    } else {
        // shouldn't be reachable
        return Err("failed to tokenize in `usb_common_case`".to_owned())
    };
    // we want to handle (r + -8), (-8 + r), (-a + -5), (-x - -y).
    // what we do is keep all chars but track all leaf '-' and '+' occurances,
    // separating when '-' are immediately adjacent.
    let mut seen_plus = Vec::<usize>::new();
    let mut seen_minus = Vec::<usize>::new();
    let mut string = Vec::<char>::new();
    // traverse the tree
    let mut stack: Vec<(VecDeque<TokenTree>, Delimiter)> =
        vec![(input.into_iter().collect(), Delimiter::None)];
    // hack to extract from single group of parenthesis
    if stack[0].0.len() == 1 {
        let tt = stack[0].0.front().unwrap();
        if let TokenTree::Group(g) = tt {
            if g.delimiter() == Delimiter::Parenthesis {
                stack[0] = (g.stream().into_iter().collect(), Delimiter::None);
            };
        }
    }
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
                    if (last == 0) && (p == '+') {
                        seen_plus.push(string.len());
                        string.push('+');
                    } else if (last == 0) && (p == '-') {
                        seen_minus.push(string.len());
                        string.push('-');
                    } else {
                        string.push(p);
                    }
                }
                TokenTree::Literal(l) => string.extend(l.to_string().chars()),
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
    let mut lhs = None;
    let mut rhs = None;
    let mut neg = false;
    if !seen_plus.is_empty() {
        lhs = Some(Usb::new_s(&string[..*seen_plus.last().unwrap()]));
        rhs = Some(Usb::new_s(&string[(*seen_plus.last().unwrap() + 1)..]));
    } else if !seen_minus.is_empty() {
        let mut mid = None;
        // search for rightmost adjacent '-'s, (a - -8) which got compressed
        for i in (0..(seen_minus.len() - 1)).rev() {
            if (seen_minus[i] + 1) == seen_minus[i + 1] {
                mid = Some(seen_minus[i]);
            }
        }
        // else just use last minus
        if mid.is_none() {
            mid = Some(*seen_minus.last().unwrap());
        }
        if let Some(mid) = mid {
            lhs = Some(Usb::new_s(&string[..mid]));
            rhs = Some(Usb::new_s(&string[(mid + 1)..]));
            neg = true;
        }
    }
    if let (Some(mut lhs), Some(mut rhs)) = (lhs, rhs) {
        // TODO need to fix quadratic terms involved
        lhs.simplify()?;
        rhs.simplify()?;
        if let Some(rhs) = rhs.static_val() {
            if neg {
                lhs.x = lhs
                    .x
                    .checked_sub(rhs)
                    .ok_or_else(|| "i128 overflow".to_owned())?;
            } else {
                lhs.x = lhs
                    .x
                    .checked_add(rhs)
                    .ok_or_else(|| "i128 overflow".to_owned())?;
            }
            lhs.x = lhs
                .x
                .checked_add(original.x)
                .ok_or_else(|| "i128 overflow".to_owned())?;
            Ok(Some(lhs))
        } else if let Some(lhs) = lhs.static_val() {
            rhs.x = rhs
                .x
                .checked_add(lhs)
                .ok_or_else(|| "i128 overflow".to_owned())?;
            if neg {
                // compiler will handle the '-' later
                rhs.s.insert(0, '-');
            }
            rhs.x = rhs
                .x
                .checked_add(original.x)
                .ok_or_else(|| "i128 overflow".to_owned())?;
            Ok(Some(rhs))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}
