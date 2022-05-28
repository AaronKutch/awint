use std::{collections::HashSet, fmt::Write, iter, num::NonZeroUsize};

use triple_arena::Ptr;

use crate::{Ast, Delimiter, PText, Text};

/// Wrap `s` in ANSI delimiters for terminal colors.
/// {90..=97} => {grey, red, green, yellow, blue, purple, cyan, white}
pub fn color_text(s: &str, c: u8) -> String {
    format!("\x1b[{}m{}\x1b[0m", c, s)
}

#[derive(Debug, Default, Clone)]
pub struct CCMacroError {
    pub red_text: Vec<Ptr<PText>>,
    pub error: String,
    pub help: Option<String>,
}

impl CCMacroError {
    pub fn new(error: String, red_text: Ptr<PText>) -> Self {
        Self {
            red_text: vec![red_text],
            error,
            help: None,
        }
    }

    /// Creates a formatted block of the cc optionally with red colored pointers
    /// to a specific concatenation or component, then `error` is appended
    /// inline.
    pub fn ast_error(&self, ast: &Ast) -> String {
        // enhance cc level punctuation
        let comma = &color_text(", ", 97);
        let semicolon = &color_text("; ", 97);
        let lbracket = &color_text("[", 97);
        let rbracket = &color_text("]", 97);
        let mut s = String::new();
        let mut color_line = String::new();
        // efficiency is not going to matter on terminal errors
        let red_text: HashSet<Ptr<PText>> = self.red_text.iter().copied().collect();

        // if some, color carets are active
        let mut color_lvl = None;
        let mut use_color_line = false;
        let extend = |color_line: &mut String, color_lvl: &Option<usize>, len: usize| {
            color_line.extend(iter::repeat(if color_lvl.is_some() { '^' } else { ' ' }).take(len))
        };
        let mut stack: Vec<(Ptr<PText>, usize)> = vec![(ast.txt_root, 0)];
        loop {
            let last = stack.len() - 1;
            if let Some(txt) = ast.txt[stack[last].0].get(stack[last].1) {
                match txt {
                    Text::Group(d, p) => {
                        if color_lvl.is_none() && red_text.contains(p) {
                            color_lvl = Some(last);
                        }
                        extend(&mut color_line, &color_lvl, d.lhs_chars().len());
                        if *d == Delimiter::RangeBracket {
                            s += lbracket;
                        } else {
                            s += d.lhs_str();
                        }
                        stack.push((*p, 0));
                        continue
                    }
                    Text::Chars(chars) => {
                        extend(&mut color_line, &color_lvl, chars.len());
                        for c in chars {
                            s.push(*c);
                        }
                        stack[last].1 += 1;
                    }
                }
            } else {
                if last == 0 {
                    break
                }
                stack.pop();
                let last = stack.len() - 1;
                let mut unset_color = false;
                if let Some(prev_last) = color_lvl {
                    if last == prev_last {
                        use_color_line = true;
                        unset_color = true;
                    }
                }
                match ast.txt[stack[last].0][stack[last].1] {
                    Text::Group(d, _) => match d {
                        Delimiter::Component => {
                            let len = ast.txt[stack[last].0].len();
                            if (stack[last].1 + 1) != len {
                                s += comma;
                                extend(&mut color_line, &color_lvl, 1);
                            }
                            if unset_color {
                                if red_text.len() == 1 {
                                    write!(
                                        color_line,
                                        " component {}: {}",
                                        len - 1 - stack[last].1,
                                        self.error
                                    )
                                    .unwrap();
                                }
                                color_lvl = None;
                            } else if (stack[last].1 + 1) != ast.txt[stack[last].0].len() {
                                extend(&mut color_line, &color_lvl, 1);
                            }
                        }
                        Delimiter::Concatenation => {
                            s += semicolon;
                            extend(&mut color_line, &color_lvl, 1);
                            if use_color_line {
                                s += &color_text(
                                    &format!("concatenation {}\n{}", stack[last].1, color_line),
                                    91,
                                );
                                use_color_line = false;
                                color_lvl = None;
                            } else {
                                extend(&mut color_line, &color_lvl, 1);
                            }
                            color_line.clear();
                            s.push('\n');
                        }
                        Delimiter::RangeBracket => {
                            s += rbracket;
                            extend(&mut color_line, &color_lvl, 1);
                        }
                        _ => {
                            s += d.rhs_str();
                            extend(&mut color_line, &color_lvl, d.rhs_str().len());
                        }
                    },
                    _ => unreachable!(),
                }
                if unset_color {
                    color_lvl = None;
                }
                stack[last].1 += 1;
            }
        }
        if let Some(ref help) = self.help {
            format!("{}\n{}{} {}", self.error, s, color_text("help:", 93), help)
        } else {
            format!("{}\n{}", self.error, s)
        }
    }
}

pub fn error_and_help(error: &str, help: &str) -> String {
    format!("{}\n{} {}", error, color_text("help:", 93), help)
}

pub fn i128_to_usize(x: i128) -> Result<usize, String> {
    usize::try_from(x).map_err(|_| "`usize::try_from` overflow".to_owned())
}

pub fn i128_to_nonzerousize(x: i128) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(i128_to_usize(x)?).ok_or_else(|| "`NonZeroUsize::new` overflow".to_owned())
}

pub fn usize_to_i128(x: usize) -> Result<i128, String> {
    i128::try_from(x).map_err(|_| "`i128::try_from` overflow".to_owned())
}
