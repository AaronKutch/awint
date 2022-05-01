use std::{collections::VecDeque, fmt::Write, mem, str::FromStr};

use proc_macro2::{Delimiter, TokenStream, TokenTree};

use crate::ranges::Usb;

/// Parses `input` `TokenStream` into "raw" concatenations of components in
/// `Vec<char>` strings
pub fn token_stream_to_raw_cc(input: TokenStream) -> Vec<Vec<Vec<char>>> {
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

pub fn raw_cc_to_string(cc: &[Vec<Vec<char>>]) -> String {
    let mut s = String::new();
    let mut concat_s = String::new();
    let mut comp_s = String::new();
    let concats_w = cc.len();
    for (j, concatenation) in cc.iter().enumerate() {
        let concat_w = concatenation.len();
        for (i, component) in concatenation.iter().enumerate() {
            for c in component {
                comp_s.push(*c);
            }
            write!(concat_s, "{}", comp_s).unwrap();
            if (i + 1) < concat_w {
                write!(concat_s, ", ").unwrap();
            }
            comp_s.clear();
        }
        write!(s, "{}", concat_s).unwrap();
        if (j + 1) < concats_w {
            writeln!(s, ";").unwrap();
        }
        concat_s.clear();
    }
    s
}

pub fn chars_to_string(chars: &[char]) -> String {
    let mut s = String::new();
    for c in chars {
        s.push(*c);
    }
    s
}

/// In ranges we commonly see stuff like `(x + y)` or `(x - y)` with one of them
/// being a contant we can parse, which passes upward the `Usb` and `Usbr` chain
/// to get calculated into a static width. This function needs to use
/// `TokenStream`s to preven breaking stuff involving nested parenthesis and
/// other stuff.
pub fn usb_common_case(input: &[char]) -> Result<Usb, String> {
    let original_input = input;
    let input = if let Ok(ts) = TokenStream::from_str(&chars_to_string(input)) {
        ts
    } else {
        // something is certainly wrong that will continue to be wrong in the code gen
        return Err("failed to tokenize".to_owned())
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
        if seen_plus.len() != 1 {
            return Ok(Usb::new(original_input, 0))
        } else {
            lhs = Some(Usb::new_s(&string[..seen_plus[0]]));
            rhs = Some(Usb::new_s(&string[(seen_plus[0] + 1)..]));
        }
    } else if !seen_minus.is_empty() {
        let mut mid = None;
        // search for rightmost adjacent '-'s, (a - -8) which got compressed
        for i in (0..(seen_minus.len() - 1)).rev() {
            if (seen_minus[i] + 1) == seen_minus[i + 1] {
                mid = Some(i);
            }
        }
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
        if (lhs.simplify().is_err() || rhs.simplify().is_err())
            || (lhs.static_val().is_none() && rhs.static_val().is_none())
        {
            Ok(Usb::new(original_input, 0))
        } else if let Some(rhs) = rhs.static_val() {
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
            Ok(lhs)
        } else {
            let lhs = lhs.static_val().unwrap();
            rhs.x
                .checked_add(lhs)
                .ok_or_else(|| "i128 overflow".to_owned())?;
            if neg {
                // compiler will handle the '-' later
                rhs.s.insert(0, '-');
            }
            Ok(rhs)
        }
    } else {
        Ok(Usb::new(original_input, 0))
    }
}
