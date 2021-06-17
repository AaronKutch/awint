use std::num::NonZeroUsize;

use awint_ext::ExtAwi;

use crate::{ComponentType::*, *};

fn remove_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Note: this function assumes that whitespace has been taken out
fn parse_range(s: &str) -> Result<Usbr, Option<String>> {
    let exclusive: Vec<String> = s.split("..").map(|s| s.to_owned()).collect();
    let inclusive: Vec<String> = s.split("..=").map(|s| s.to_owned()).collect();
    if exclusive.len() > 2 || inclusive.len() > 2 {
        return Err(Some("too many ranges".to_owned()))
    }
    if exclusive.len() == 2 {
        // at least ".."
        if inclusive.len() == 2 {
            // "..="
            let start = if inclusive[0].is_empty() {
                None
            } else {
                Some(Usb::new(&inclusive[0], 0))
            };
            let end = if inclusive[1].is_empty() {
                None
            } else {
                Some(Usb::new(&inclusive[1], 1))
            };
            Ok(Usbr { start, end })
        } else {
            let start = if exclusive[0].is_empty() {
                None
            } else {
                Some(Usb::new(&exclusive[0], 0))
            };
            let end = if exclusive[1].is_empty() {
                None
            } else {
                Some(Usb::new(&exclusive[1], 0))
            };
            Ok(Usbr { start, end })
        }
    } else {
        Err(None)
    }
}

fn parse_component(nw: &[u8]) -> Result<Component, String> {
    // if the for loop does not find an index, the name defaults to the whole `nw`
    // and the range is unbounded
    let mut name = String::from_utf8(nw.to_owned()).unwrap();
    let mut index: Option<Usbr> = None;
    for j0 in 0..nw.len() {
        // see if there is an index
        if nw[j0] == b'[' {
            if nw[nw.len() - 1] != b']' {
                return Err("has an opening '[' but not a closing ']'".to_owned())
            }
            name = String::from_utf8(Vec::<u8>::from(&nw[0..j0])).unwrap();
            // get pattern split ability back
            let nw = String::from_utf8(Vec::<u8>::from(&nw[(j0 + 1)..(nw.len() - 1)])).unwrap();
            if nw.as_bytes().is_empty() {
                return Err("has an empty index".to_owned())
            }
            match parse_range(&nw) {
                Ok(range) => {
                    index = Some(range);
                }
                Err(Some(e)) => return Err(e),
                Err(None) => {
                    // with no detected range, we assume the index is getting a single bit
                    index = Some(Usbr::single_bit(&nw));
                }
            }
        }
    }
    // If there was no index, check if this component is a filler range. If the
    // index is still `None`, assign an `unbounded_range()` to treat the variable or
    // literal as having "[..]" appended.
    let index = match index {
        Some(range) => range,
        None => match parse_range(&name) {
            Ok(filler_range) => {
                name.clear();
                filler_range
            }
            Err(Some(e)) => return Err(e),
            Err(None) => Usbr::unbounded(),
        },
    };
    let mut component = match name.as_bytes().iter().next() {
        Some(b'-' | b'0'..=b'9') => {
            // assume literal
            match name.parse::<ExtAwi>() {
                Ok(awi) => Component::new(Literal(awi), index),
                Err(e) => {
                    return Err(format!(
                        "was parsed with `<ExtAwi as FromStr>::from_str(\"{}\")` which returned \
                         SerdeError::{:?}",
                        name, e
                    ))
                }
            }
        }
        Some(_) => {
            // assume variable
            Component::new(Variable(name), index)
        }
        None => {
            // assume filler
            Component::new(Filler, index)
        }
    };
    if let Err(e) = component.attempt_simplify() {
        return Err(e)
    }
    Ok(component)
}

pub(crate) fn parse_concatenation(
    concatenation: &str,
    is_sink: bool,
    id: &mut usize,
) -> Result<Concatenation, String> {
    let input_components = concatenation
        .to_string()
        .split(',')
        .rev() // Note the reversal here for big endianness
        .map(|s| s.to_owned())
        .collect::<Vec<String>>();
    let mut components: Vec<Component> = Vec::new();
    // We start by assuming that we can calculate a static width. This gets set to
    // `None` if we encounter a variable range, or an unbounded range on a
    // variable.
    let mut total_bw: Option<usize> = Some(0);
    let mut unbounded_filler = false;
    for (i, component) in input_components.iter().enumerate() {
        // TODO we remove whitespace because a space is inserted between negatives "-"
        // and their literals. We should use `syn` to be more strict about whitespace.
        // We should also use `syn` to be more flexible such as allowing comments.
        let nw = remove_whitespace(component);
        if !nw.is_ascii() {
            return Err(format!("component {} (\"{}\"): is not ascii", i, component))
        }
        let nw: Vec<u8> = nw.bytes().collect();
        if nw.is_empty() {
            if i == 0 {
                continue
            } else {
                return Err(format!("component {}: is empty or only whitespace", i))
            }
        }
        match parse_component(&nw) {
            Ok(c) => {
                match c.component_type {
                    Filler => {
                        if c.range.end.is_none() {
                            if unbounded_filler {
                                return Err("there is more than one unbounded filler".to_owned())
                            } else {
                                unbounded_filler = true;
                            }
                        }
                    }
                    Literal(..) => {
                        if is_sink {
                            return Err("sink concatenations cannot have literals".to_owned())
                        }
                    }
                    _ => (),
                }
                if let Some(width) = c.range.static_width() {
                    if let Some(ref mut total_bw) = total_bw {
                        *total_bw = total_bw.checked_add(width).unwrap();
                    }
                } else {
                    total_bw = None;
                }
                components.push(c);
            }
            Err(e) => return Err(format!("component {} (\"{}\"): {}", i, component, e)),
        }
        *id += 1;
    }
    let total_bw = if let Some(total_bw) = total_bw {
        match NonZeroUsize::new(total_bw) {
            Some(x) => Some(x),
            None => return Err("determined statically that this has zero bitwidth".to_owned()),
        }
    } else {
        None
    };

    // To allow grouping constants together into the same constant without
    // dramatically increasing the complexity of the code gen part, we attempt to
    // merge neighboring constants here. The truncation of the constants was already
    // handled earlier in component constification, and the ranges have already been
    // normalized to start at 0 and end at the end of the literal bitwidth
    let mut i = components.len() - 1;
    while i > 0 {
        if components[i - 1].is_static_literal() && components[i].is_static_literal() {
            // this is infallible, the only reason for this awkward arrangement is to get
            // around borrowing issues
            if let (Literal(lit0), Literal(lit1)) = (
                components[i - 1].component_type.clone(),
                components[i].component_type.clone(),
            ) {
                let w0 = components[i - 1].range.static_width().unwrap();
                let w1 = components[i].range.static_width().unwrap();
                let total = w0.checked_add(w1).unwrap();
                let mut combined = ExtAwi::zero(NonZeroUsize::new(total).unwrap());
                combined[..].zero_resize_assign(&lit0[..]);
                combined[..].field(w0, &lit1[..], 0, w1).unwrap();
                components[i - 1].component_type = Literal(combined);
                components[i - 1].range = Usbr::new_static(0, total);
                components.remove(i);
            }
        }
        i -= 1;
    }

    Ok(Concatenation {
        concatenation: components,
        total_bw,
    })
}

pub(crate) fn parse_concats(input: &str) -> Result<Vec<Concatenation>, String> {
    let tmp = remove_whitespace(input);
    if tmp.is_empty() {
        return Err("input is empty or only whitespace".to_owned())
    }
    let concats: Vec<String> = input.to_string().split(';').map(|s| s.to_owned()).collect();
    let mut output = Vec::new();
    let mut id = 0;
    for (i, concatenation) in concats.iter().enumerate() {
        let tmp = remove_whitespace(concatenation);
        if tmp.is_empty() {
            if i == (concats.len() - 1) {
                // allow trailing semicolon
                continue
            } else {
                return Err(format!("concatenation {}: is empty or only whitespace", i))
            }
        }
        match parse_concatenation(concatenation, i != 0, &mut id) {
            Ok(v) => {
                output.push(v);
            }
            Err(e) => {
                return Err(format!(
                    "concatenation {} (\"{}\"): {}",
                    i, concatenation, e
                ))
            }
        }
    }
    Ok(output)
}