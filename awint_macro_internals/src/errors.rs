use std::fmt::Write;

/// Wrap `s` in ANSI delimiters for terminal colors.
/// {90..=97} => {grey, red, green, yellow, blue, purple, cyan, white}
pub fn color_text(s: &str, c: u8) -> String {
    format!("\x1b[{}m{}\x1b[0m", c, s)
}

pub struct CCMacroError {
    pub concat_i: Option<usize>,
    pub comp_i: Option<usize>,
    pub error: String,
}

impl CCMacroError {
    pub fn new(error: String) -> Self {
        Self {
            concat_i: None,
            comp_i: None,
            error,
        }
    }

    /// Creates a formatted block of the cc optionally with red colored pointers
    /// to a specific concatenation or component, then `error` is appended
    /// inline.
    pub fn raw_cc_error(&self, cc: &[Vec<Vec<char>>]) -> String {
        // enhance cc level punctuation
        let comma = &color_text(",", 97);
        let semicolon = &color_text(";", 97);
        let mut s = String::new();
        let mut concat_s = String::new();
        let mut comp_s = String::new();
        let mut color_line = String::new();
        for (j, concatenation) in cc.iter().enumerate() {
            let mut this_concat = false;
            let mut mark = false;
            if let Some(concat_i) = self.concat_i {
                if j == concat_i {
                    this_concat = true;
                    if self.comp_i.is_none() {
                        mark = true;
                    }
                }
            }
            let mut advance = this_concat;
            for (i, component) in concatenation.iter().enumerate().rev() {
                if this_concat && advance {
                    if let Some(comp_i) = self.comp_i {
                        if i == comp_i {
                            mark = true;
                            advance = false;
                        }
                    }
                    let parallel_c = if mark { '^' } else { ' ' };
                    for _ in 0..component.len() {
                        color_line.push(parallel_c);
                    }
                    if self.comp_i.is_some() {
                        mark = false;
                    }
                }
                for c in component {
                    comp_s.push(*c);
                }
                write!(concat_s, "{}", comp_s).unwrap();
                if i > 0 {
                    write!(concat_s, "{} ", comma).unwrap();
                }
                comp_s.clear();
            }
            write!(s, "{}{}", concat_s, semicolon).unwrap();
            concat_s.clear();
            if this_concat {
                if let Some(comp_i) = self.comp_i {
                    color_line = format!(
                        " concatenation {}\n{} component {}: {}",
                        j, color_line, comp_i, self.error
                    );
                } else {
                    color_line = format!(" concatenation {}\n{} {}", j, color_line, self.error);
                }
                writeln!(s, "{}", color_text(&color_line, 91)).unwrap();
            }
            writeln!(s).unwrap();
        }
        format!("{}\n{}", self.error, s)
    }
}
