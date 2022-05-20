use std::num::NonZeroUsize;

use awint_ext::ExtAwi;

use crate::{
    component::{Component, ComponentType::*},
    i128_to_nonzerousize, i128_to_usize,
    ranges::Usbr,
    Ast, CCMacroError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillerAlign {
    /// The concatenation has no fillers
    None,
    /// The concatenation can be aligned from the least significant bit
    Lsb,
    /// The concatenation can be aligned from the most significant bit
    Msb,
    /// The filler is in the middle
    Mid,
    /// For multiple concatenations, multiple alignments
    Multiple,
}

#[derive(Debug, Clone)]
pub struct Concatenation {
    pub comps: Vec<Component>,
    pub total_bw: Option<NonZeroUsize>,
    pub filler_alignment: FillerAlign,
    // even if `total_bw` is not statically known, this concatenation width could be known from
    // this concatentation alone at runtime
    pub deterministic_width: bool,
}

impl Concatenation {
    pub fn is_empty(&self, ast: &Ast) -> bool {
        (self.comps.len() == 1) && ast.text[self.comps[0].text].is_empty()
    }

    pub fn check(&mut self, concat_i: usize) -> Result<(), CCMacroError> {
        let concat_len = self.comps.len();
        let mut cumulative_bw = Some(0usize);
        // start by assuming yes
        self.deterministic_width = true;
        for (comp_i, comp) in self.comps.iter().enumerate() {
            if let Some(var_w) = comp.range.static_width() {
                if let Some(ref mut w) = cumulative_bw {
                    *w = w.checked_add(i128_to_usize(var_w).unwrap()).unwrap();
                }
            } else {
                cumulative_bw = None;
            }
            if comp.range.end.is_none() {
                self.deterministic_width = false;
            }
            match comp.c_type {
                Unparsed => unreachable!(),
                Literal(_) => {
                    if concat_i > 0 {
                        return Err(CCMacroError {
                            concat_i: Some(concat_i),
                            comp_i: Some(comp_i),
                            error: "sink concatenations cannot have literals".to_owned(),
                            help: Some(
                                "if the space taken up by the component is necessary, use a \
                                 filler equivalent to its width or range instead"
                                    .to_owned(),
                            ),
                        })
                    }
                }
                Variable(_) => {}
                Filler => {
                    // unbounded filler handling
                    if comp.range.end.is_none() {
                        if (concat_i != 0) && (concat_len == 0) {
                            return Err(CCMacroError {
                                concat_i: Some(concat_i),
                                error: "sink concatenations that consist of only an unbounded \
                                        filler are no-ops"
                                    .to_owned(),
                                ..Default::default()
                            })
                        }
                        if !matches!(self.filler_alignment, FillerAlign::None) {
                            // filler already set
                            return Err(CCMacroError {
                                concat_i: Some(concat_i),
                                // lets point to one
                                comp_i: Some(comp_i),
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
                            self.filler_alignment = FillerAlign::Lsb;
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
                self.total_bw = Some(w);
            } else {
                // in the case of `cc!` this isn't a logical error, but it is a useless no-op
                return Err(CCMacroError {
                    concat_i: Some(concat_i),
                    error: "determined statically that this concatenation has zero width"
                        .to_owned(),
                    help: Some(
                        "if this is a construction macro then it would result in a zero bitwidth \
                         `awint` integer which would panic, else it is still a useless no-op"
                            .to_owned(),
                    ),
                    ..Default::default()
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
                    combined.zero_resize_assign(&lit0);
                    combined
                        .field(
                            i128_to_usize(w0).unwrap(),
                            &lit1,
                            0,
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

pub fn parse_cc(raw_cc: &[Vec<Vec<char>>]) -> Result<Vec<Concatenation>, CCMacroError> {
    let mut cc = vec![];
    for (concat_i, concat) in raw_cc.iter().enumerate() {
        let mut comps = vec![];
        for (comp_i, comp) in concat.iter().enumerate() {
            /*match parse_component(comp, false) {
                Ok((Some(_), _)) => todo!(),
                Ok((None, comp)) => comps.push(comp),
                Err(e) => {
                    return Err(CCMacroError {
                        concat_i: Some(concat_i),
                        comp_i: Some(comp_i),
                        error: e,
                        ..Default::default()
                    })
                }
            }*/
        }
        cc.push(Concatenation {
            comps,
            total_bw: None,
            filler_alignment: FillerAlign::None,
            deterministic_width: false,
        });
    }
    Ok(cc)
}

pub fn stage2(cc: &mut [Concatenation]) -> Result<(), CCMacroError> {
    for (concat_i, concat) in cc.iter_mut().enumerate() {
        for (comp_i, comp) in concat.comps.iter_mut().enumerate() {
            match comp.simplify() {
                Ok(()) => (),
                Err(e) => {
                    return Err(CCMacroError {
                        concat_i: Some(concat_i),
                        comp_i: Some(comp_i),
                        error: e,
                        ..Default::default()
                    })
                }
            }
        }
    }
    Ok(())
}

pub fn stage3(cc: &mut [Concatenation]) -> Result<(), CCMacroError> {
    for (concat_i, concat) in cc.iter_mut().enumerate() {
        concat.check(concat_i)?;
    }
    Ok(())
}

pub fn stage4(
    cc: &mut [Concatenation],
    specified_init: bool,
    return_type: Option<&str>,
    static_width: bool,
) -> Result<(), CCMacroError> {
    let mut overall_alignment = cc[0].filler_alignment;
    let mut alignment_change_i = 0;
    let mut all_deterministic = cc[0].deterministic_width;
    let mut common_bw = cc[0].total_bw;
    let mut original_common_i = 0;
    for (concat_i, concat) in cc.iter().enumerate() {
        let this_align = concat.filler_alignment;
        match this_align {
            FillerAlign::None | FillerAlign::Multiple => (),
            FillerAlign::Lsb | FillerAlign::Msb | FillerAlign::Mid => {
                if matches!(overall_alignment, FillerAlign::None) {
                    overall_alignment = this_align
                } else if overall_alignment != this_align {
                    alignment_change_i = concat_i;
                    overall_alignment = FillerAlign::Multiple
                }
            }
        }
        all_deterministic |= concat.deterministic_width;
        if let Some(this_bw) = concat.total_bw {
            if let Some(prev_bw) = common_bw {
                if this_bw != prev_bw {
                    return Err(CCMacroError {
                        concat_i: Some(concat_i),
                        error: format!(
                            "determined statically that concatenations {} and {} have unequal \
                             bitwidths {} and {}",
                            original_common_i, concat_i, prev_bw, this_bw
                        ),
                        ..Default::default()
                    })
                }
            } else {
                common_bw = Some(this_bw);
                original_common_i = concat_i;
            }
        }
        if (!specified_init) && (concat_i == 0) {
            for (comp_i, comp) in concat.comps.iter().enumerate() {
                if matches!(comp.c_type, Filler) {
                    return Err(CCMacroError {
                        concat_i: Some(concat_i),
                        comp_i: Some(comp_i),
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
    if (!all_deterministic) && (cc.len() == 1) {
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
    if !all_deterministic {
        for (concat_i, concat) in cc.iter().enumerate() {
            if matches!(concat.filler_alignment, FillerAlign::Mid) {
                for (comp_i, comp) in concat.comps.iter().enumerate() {
                    if matches!(comp.c_type, Filler) && comp.range.end.is_none() {
                        return Err(CCMacroError {
                            concat_i: Some(concat_i),
                            comp_i: Some(comp_i),
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
    if (!all_deterministic) && matches!(overall_alignment, FillerAlign::Multiple) {
        // note: middle fillers have been accounted for, only opposite alignment
        // possible at this point
        for (concat_i, concat) in cc.iter().enumerate() {
            if !matches!(concat.filler_alignment, FillerAlign::None) {
                return Err(CCMacroError {
                    concat_i: Some(alignment_change_i),
                    error: format!(
                        "concatenations {} and {} have unbounded fillers aligned opposite each \
                         other, and no concatenation has a statically or dynamically determinable \
                         width",
                        concat_i, alignment_change_i
                    ),
                    help: Some(
                        "append a filler-only concatenation such as \"; ..64 ;\" or \"; ..var ;\" \
                         that gives the macro needed information"
                            .to_owned(),
                    ),
                    ..Default::default()
                })
            }
        }
    }
    Ok(())
}

pub fn stage5(cc: &mut [Concatenation]) {
    for concat in cc {
        concat.simplify();
    }
}
