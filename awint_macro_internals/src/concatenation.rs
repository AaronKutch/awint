use std::num::NonZeroUsize;

use awint_ext::ExtAwi;

use crate::{
    i128_to_nonzerousize, i128_to_usize, Ast, CCMacroError, Component, ComponentType::*, PCWidth,
    PText, Usbr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillerAlign {
    /// The concatenation has no unbounded fillers, or in the case of multiple
    /// concatenations there is a mix of the `None` cases and `Single | Lsb |
    /// Msb | Mid | Multiple` cases.
    None,
    /// If the concatenation is only a filler
    Single,
    /// The concatenation can be aligned from the least significant bit
    Lsb,
    /// The concatenation can be aligned from the most significant bit
    Msb,
    /// The filler is in the middle
    Mid,
    /// For multiple concatenations, all are not `None` and there are two or
    /// more of `Lsb | Msb | Mid`
    Multiple,
}

impl Default for FillerAlign {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Default, Clone)]
pub struct Concatenation {
    pub txt: PText,
    pub comps: Vec<Component>,
    pub filler_alignment: FillerAlign,
    pub static_width: Option<NonZeroUsize>,
    // even if `total_bw` is not statically known, this concatenation width could be known from
    // this concatentation alone at runtime
    pub deterministic_width: bool,
    pub guaranteed_nonzero_width: bool,
    // concatenation width
    pub cw: Option<PCWidth>,
}

impl Concatenation {
    pub fn check(&mut self, concat_i: usize, concat_txt: PText) -> Result<(), CCMacroError> {
        let concat_len = self.comps.len();
        // start by assuming yes
        let mut cumulative_bw = Some(0usize);
        self.deterministic_width = true;
        for (comp_i, comp) in self.comps.iter().enumerate() {
            if let Some(var_w) = comp.range.static_width() {
                // zero static widths not allowed
                self.guaranteed_nonzero_width = true;
                if let Some(ref mut w) = cumulative_bw {
                    *w = w.checked_add(i128_to_usize(var_w).unwrap()).unwrap();
                }
            } else {
                cumulative_bw = None;
            }
            match comp.c_type {
                Unparsed => unreachable!(),
                Literal(_) => {
                    if concat_i > 0 {
                        return Err(CCMacroError {
                            red_text: vec![comp.txt],
                            error: "sink concatenations cannot have literals".to_owned(),
                            help: Some(
                                "if the space taken up by the component is necessary, use a \
                                 filler equivalent to its width instead"
                                    .to_owned(),
                            ),
                        })
                    }
                }
                Variable => {
                    if comp.has_full_range() {
                        self.guaranteed_nonzero_width = true;
                    }
                }
                Filler => {
                    // unbounded filler handling
                    if comp.range.end.is_none() {
                        self.deterministic_width = false;
                        if (concat_i != 0) && (concat_len == 1) {
                            return Err(CCMacroError::new(
                                "sink concatenations that consist of only an unbounded filler are \
                                 no-ops"
                                    .to_owned(),
                                concat_txt,
                            ))
                        }
                        if !matches!(self.filler_alignment, FillerAlign::None) {
                            // filler already set
                            return Err(CCMacroError {
                                // lets point to one
                                red_text: vec![comp.txt],
                                // be explicit
                                error: "there is more than one unbounded filler in this \
                                        concatenation"
                                    .to_owned(),
                                help: Some(
                                    "it is ambiguous how components between the fillers should be \
                                     aligned, remove one or break apart the macro into more macros"
                                        .to_owned(),
                                ),
                            })
                        }
                        if comp_i == 0 {
                            if concat_len == 1 {
                                self.filler_alignment = FillerAlign::Single;
                            } else {
                                self.filler_alignment = FillerAlign::Lsb;
                            }
                        } else if (comp_i + 1) == concat_len {
                            self.filler_alignment = FillerAlign::Msb;
                        } else {
                            self.filler_alignment = FillerAlign::Mid;
                        }
                    }
                }
            }
        }
        if let Some(w) = cumulative_bw {
            if let Some(w) = NonZeroUsize::new(w) {
                self.static_width = Some(w);
            } else {
                // not sure if this is reachable
                return Err(CCMacroError {
                    red_text: vec![concat_txt],
                    error: "determined statically that this concatenation has zero width"
                        .to_owned(),
                    help: Some(
                        "if this is a construction macro then it would result in a zero bitwidth \
                         `awint` integer which would panic, else it is still a useless no-op"
                            .to_owned(),
                    ),
                })
            }
        }
        Ok(())
    }

    /// Should be run after all checks, because it combines neighboring static
    /// constants
    pub fn simplify(&mut self) {
        // To allow grouping constants together into the same constant without
        // dramatically increasing the complexity of the code gen part, we attempt to
        // merge neighboring constants here. The truncation of the constants was already
        // handled earlier in component constification, and the ranges have already been
        // normalized to start at 0 and end at the end of the literal bitwidth
        let mut i = self.comps.len() - 1;
        while i > 0 {
            if self.comps[i - 1].is_static_literal() && self.comps[i].is_static_literal() {
                // this is infallible, the only reason for this awkward arrangement is to get
                // around borrowing issues
                if let (Literal(lit0), Literal(lit1)) = (
                    self.comps[i - 1].c_type.clone(),
                    self.comps[i].c_type.clone(),
                ) {
                    let w0 = self.comps[i - 1].range.static_width().unwrap();
                    let w1 = self.comps[i].range.static_width().unwrap();
                    let total_i128 = w0.checked_add(w1).unwrap();
                    let total = i128_to_nonzerousize(total_i128).unwrap();
                    let mut combined = ExtAwi::zero(total);
                    combined.zero_resize_(&lit0);
                    combined
                        .field_to(
                            i128_to_usize(w0).unwrap(),
                            &lit1,
                            i128_to_usize(w1).unwrap(),
                        )
                        .unwrap();
                    self.comps[i - 1].c_type = Literal(combined);
                    self.comps[i - 1].range = Usbr::new_static(0, total_i128);
                    self.comps.remove(i);
                }
            }
            i -= 1;
        }
    }
}

pub fn stage3(ast: &mut Ast) -> Result<(), CCMacroError> {
    for (concat_i, concat) in ast.cc.iter_mut().enumerate() {
        concat.check(concat_i, concat.txt)?;
    }
    Ok(())
}

pub fn stage4(
    ast: &mut Ast,
    return_type: &Option<&str>,
    static_width: bool,
) -> Result<(), CCMacroError> {
    let mut overall_alignment = ast.cc[0].filler_alignment;
    let mut deterministic = ast.cc[0].deterministic_width;
    let mut common_bw = ast.cc[0].static_width;
    let mut original_common_i = 0;
    for (concat_i, concat) in ast.cc.iter().enumerate() {
        ast.guaranteed_nonzero_width |= concat.guaranteed_nonzero_width;
        let this_align = concat.filler_alignment;
        if this_align == FillerAlign::None {
            overall_alignment = FillerAlign::None;
        } else {
            match overall_alignment {
                FillerAlign::None => (),
                FillerAlign::Single => {
                    overall_alignment = this_align;
                }
                align @ (FillerAlign::Lsb | FillerAlign::Msb | FillerAlign::Mid) => {
                    if (this_align != FillerAlign::Single) && (this_align != align) {
                        overall_alignment = FillerAlign::Multiple
                    }
                }
                FillerAlign::Multiple => (),
            }
        }
        deterministic |= concat.deterministic_width;
        if let Some(this_bw) = concat.static_width {
            if let Some(prev_bw) = common_bw {
                if this_bw != prev_bw {
                    return Err(CCMacroError::new(
                        format!(
                            "determined statically that concatenations {original_common_i} and \
                             {concat_i} have unequal bitwidths {prev_bw} and {this_bw}"
                        ),
                        concat.txt,
                    ))
                }
            } else {
                common_bw = Some(this_bw);
                original_common_i = concat_i;
            }
        }
        if ast.txt_init.is_none() && (concat_i == 0) && return_type.is_some() {
            for comp in &concat.comps {
                if matches!(comp.c_type, Filler) {
                    return Err(CCMacroError {
                        red_text: vec![comp.txt],
                        error: "a construction macro with unspecified initialization cannot have \
                                a filler in the source concatenation"
                            .to_owned(),
                        help: Some(
                            "prefix the first concatenation with the desired initialization \
                             function followed by a colon, such as \"zero: \" or \"umax: \""
                                .to_owned(),
                        ),
                    })
                }
            }
        }
    }
    if static_width && common_bw.is_none() {
        return Err(CCMacroError {
            error: format!(
                "`{}` construction macros need at least one concatenation to have a width that \
                 can be determined statically by the macro",
                return_type.unwrap()
            ),
            help: Some(
                "use constant ranges on all the components of any concatenation, or append a \
                 filler-only concatenation such as \"; ..64 ;\" that gives the macro needed \
                 information"
                    .to_owned(),
            ),
            ..Default::default()
        })
    }
    if (!deterministic) && (ast.cc.len() == 1) {
        // this case shouldn't have a use
        return Err(CCMacroError {
            error: "there is a only a source concatenation that has no statically or dynamically \
                    determinable width"
                .to_owned(),
            help: Some(
                "unbounded fillers have no effects if there is only one concatenation".to_owned(),
            ),
            ..Default::default()
        })
    }
    if !deterministic {
        for concat in &ast.cc {
            if matches!(concat.filler_alignment, FillerAlign::Mid) {
                for comp in &concat.comps {
                    if matches!(comp.c_type, Filler) && comp.range.end.is_none() {
                        return Err(CCMacroError {
                            red_text: vec![comp.txt],
                            error: "there is an unbounded filler in the middle of a \
                                    concatenation, and no concatenation has a statically or \
                                    dynamically determinable width"
                                .to_owned(),
                            help: Some(
                                "append a filler-only concatenation such as \"; ..64 ;\" or \"; \
                                 ..var ;\" that gives the macro needed information"
                                    .to_owned(),
                            ),
                        })
                    }
                }
            }
        }
    }
    if (!deterministic) && matches!(overall_alignment, FillerAlign::Multiple) {
        // note: middle fillers have been accounted for, only opposite alignment
        // possible at this point
        for concat_i in 0..ast.cc.len() {
            let i_filler = ast.cc[concat_i].filler_alignment;
            if !matches!(i_filler, FillerAlign::None | FillerAlign::Single) {
                for concat_j in (concat_i + 1)..ast.cc.len() {
                    let j_filler = ast.cc[concat_j].filler_alignment;
                    if !matches!(j_filler, FillerAlign::None | FillerAlign::Single)
                        && (i_filler != j_filler)
                    {
                        return Err(CCMacroError {
                            red_text: vec![],
                            error: format!(
                                "concatenations {concat_i} and {concat_j} have unbounded fillers \
                                 aligned opposite each other, and no concatenation has a \
                                 statically or dynamically determinable width"
                            ),
                            help: Some(
                                "append a filler-only concatenation such as \"; ..64 ;\" or \"; \
                                 ..var ;\" that gives the macro needed information"
                                    .to_owned(),
                            ),
                        })
                    }
                }
            }
        }
    }
    ast.common_bw = common_bw;
    ast.deterministic_width = deterministic;
    ast.overall_alignment = overall_alignment;
    Ok(())
}

pub fn stage5(ast: &mut Ast) {
    for concat in &mut ast.cc {
        concat.simplify();
    }
}
