use std::{collections::HashSet, iter, num::NonZeroUsize};

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
    pub fn new(error: String) -> Self {
        Self {
            red_text: vec![],
            error,
            help: None,
        }
    }

    pub fn new_with_text(error: String, red_text: Ptr<PText>) -> Self {
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
        let mut s = String::new();
        let mut concat_s = String::new();
        let mut comp_s = String::new();
        let mut color_line = String::new();
        let red_text: HashSet<Ptr<PText>> = self.red_text.iter().map(|p| *p).collect();

        let mut color_lvl = None;
        let mut use_color_line = false;
        let mut activated = false;
        let mut stack: Vec<(Ptr<PText>, usize)> = vec![(ast.text_root, 0)];
        loop {
            let last = stack.len() - 1;
            if let Some(text) = ast.text[stack[last].0].get(stack[last].1) {
                match text {
                    Text::Group(d, p) => {
                        if activated {
                            color_line.extend(
                                iter::repeat(if color_lvl.is_some() { '^' } else { ' ' })
                                    .take(d.lhs_text().len()),
                            );
                        }
                        s += d.lhs_text();
                        stack.push((*p, 0));
                        if red_text.contains(p) {
                            color_lvl = Some(last);
                        }
                        continue
                    }
                    Text::Chars(chars) => {
                        if activated {
                            color_line.extend(
                                iter::repeat(if color_lvl.is_some() { '^' } else { ' ' })
                                    .take(chars.len()),
                            );
                        }
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
                if red_text.contains(&stack[last].0) {
                    color_lvl = None;
                    use_color_line = true;
                    unset_color = true;
                }
                match ast.text[stack[last].0][stack[last].1] {
                    Text::Group(d, _) => match d {
                        Delimiter::Component => {
                            s += comma;
                            if unset_color {
                                if red_text.len() == 1 {
                                    color_line += &format!(" component {}: {}", stack[last].1, self.error);
                                    activated = true;
                                }
                            }
                        }
                        Delimiter::Concatenation => {
                            s += semicolon;
                            if use_color_line {
                                s += &color_text(&format!("concatenation {}\n{}", stack[last].1, color_line), 91);
                            }
                            s.push('\n');
                        }
                        _ => s += d.rhs_text(),
                    },
                    _ => unreachable!(),
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
