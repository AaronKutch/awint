// Used by `tests/macro_fuzzing.rs`.
// Here, we try to generate code which tests all successful code generation
// paths

use std::{
    cmp::{max, min},
    env,
    fs,
    fs::OpenOptions,
    io::Write,
    num::NonZeroUsize,
    path::PathBuf,
};

use awint::prelude::*;
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};

// Note: I would like 1000 tests, but the compiler on GitHub
// Actions is acting up and returning exit code 255

// number of tests generated
const NUM_TESTS: usize = 300;
// should be plenty to test all edge cases
const MAX_CONCATS: usize = 4;
// enough to get multiple components on each side of an unbounded filler
const MAX_COMPS: usize = 5;
// max total bitwidth. needs to be low so that special cases manifest
const MAX_BW: usize = 5 * (MAX_COMPS as usize);

#[derive(Debug, Clone, Copy)]
enum Align {
    Ls,
    Ms,
    Any,
}

impl Align {
    pub fn is_ls(&self) -> bool {
        matches!(self, Align::Ls)
    }

    pub fn is_ms(&self) -> bool {
        matches!(self, Align::Ms)
    }

    pub fn is_any(&self) -> bool {
        matches!(self, Align::Any)
    }
}

#[derive(Debug)]
struct Concat<'a> {
    /// The value. Filler bits are zeroed
    pub val: ExtAwi,
    /// Filler mask
    pub fill: ExtAwi,
    /// Alignment side, `false` is least significant
    pub align_side: bool,
    /// Least significant shift position
    pub ls_shift: usize,
    /// More significant shift position
    pub ms_shift: usize,
    /// The RNG
    rng: &'a mut Xoshiro128StarStar,
    /// The unique variable counter
    vnum: &'a mut u64,
    /// Variable setting prior to macro execution
    pub prior_sets: &'a mut String,
    pub assertions: String,
    pub dynamic_width: bool,
    pub static_width: bool,
    pub align: Align,
    pub specified_initialization: bool,
    pub only_one_concat: bool,
    pub num_comps: usize,
    pub remaining_width: usize,
    // the two sides are divided by any unbounded filer
    ls_comps: Vec<String>,
    pub unbounded: bool,
    pub non_unbounded_width: usize,
    ms_comps: Vec<String>,
}

impl<'a> Concat<'a> {
    /// Note: this starts with `align == true`
    pub fn new(
        bw: NonZeroUsize,
        rng: &'a mut Xoshiro128StarStar,
        vnum: &'a mut u64,
        prior_sets: &'a mut String,
        dynamic_width: bool,
        static_width: bool,
        align: Align,
        specified_initialization: bool,
        only_one_concat: bool,
    ) -> Self {
        let num_comps = min(
            (((rng.next_u32() as usize) % MAX_COMPS) + 1) as usize,
            bw.get(),
        );
        Self {
            val: ExtAwi::zero(bw),
            fill: ExtAwi::zero(bw),
            align_side: false,
            ls_shift: 0,
            ms_shift: bw.get(),
            rng,
            vnum,
            prior_sets,
            assertions: String::new(),
            dynamic_width: dynamic_width || static_width,
            static_width,
            align,
            specified_initialization,
            only_one_concat,
            num_comps,
            remaining_width: bw.get(),
            ls_comps: Vec::new(),
            unbounded: false,
            non_unbounded_width: 0,
            ms_comps: Vec::new(),
        }
    }

    pub fn next_unique(&mut self) -> u64 {
        let tmp = *self.vnum;
        *self.vnum += 1;
        tmp
    }

    pub fn rand_bool(&mut self) -> bool {
        (self.rng.next_u32() & 1) == 0
    }

    pub fn rand_usize(&mut self) -> usize {
        self.rng.next_u32() as usize
    }

    pub fn push_comp(&mut self, s: String) {
        if self.align_side {
            self.ms_comps.push(s);
        } else {
            self.ls_comps.push(s);
        }
    }

    pub fn get_concat_s(&self) -> String {
        let mut s = String::new();
        for i in 0..self.ms_comps.len() {
            s += &self.ms_comps[i];
            s += ", ";
        }
        for i in (0..self.ls_comps.len()).rev() {
            s += &self.ls_comps[i];
            s += ", ";
        }
        s
    }

    pub fn push_awi(&mut self, val: ExtAwi, fill: ExtAwi) {
        assert_eq!(val.bw(), fill.bw());
        if self.align_side {
            self.ms_shift -= val.bw();
            self.val[..].field(self.ms_shift, &val[..], 0, val.bw());
            self.fill[..].field(self.ms_shift, &fill[..], 0, fill.bw());
        } else {
            self.val[..].field(self.ls_shift, &val[..], 0, val.bw());
            self.fill[..].field(self.ls_shift, &fill[..], 0, fill.bw());
            self.ls_shift += val.bw();
        }
    }

    /// For setting the unbounded filler bits
    pub fn set_middle_filler_mask(&mut self) {
        if self.ls_shift == self.ms_shift {
            return
        }
        let tmp = ExtAwi::umax(bw(self.ms_shift - self.ls_shift));
        self.fill[..]
            .field(self.ls_shift, &tmp[..], 0, tmp.bw())
            .unwrap();
    }

    /// The first element is the bitwidth used by the macro, second it the
    /// actual bitwidth of the referenced component
    pub fn gen_comp_bitwidth(&mut self, comp_i: usize) -> (usize, usize) {
        // we have to use up the total `width`. This also makes sure there are enough
        // single bits left
        let bitwidth = if comp_i == (self.num_comps - 1) {
            self.remaining_width
        } else {
            // make sure there is at least a bit for every component remaining
            let limiter = self.remaining_width - (self.num_comps - comp_i);
            ((self.rand_usize() % ((2 * self.remaining_width) / (self.num_comps - comp_i)))
                % limiter)
                + 1
        };
        self.remaining_width -= bitwidth;
        let referenced_bw = if self.rand_bool() {
            // full range
            bitwidth
        } else {
            // make the bitwidth of the referenced component larger
            bitwidth + (self.rand_usize() % bitwidth)
        };
        (bitwidth, referenced_bw)
    }

    /// Generate string representing range
    pub fn gen_range(
        &mut self,
        range: (Option<usize>, Option<usize>),
    ) -> (Option<String>, Option<String>, bool) {
        let inclusive = self.rand_bool() as usize;
        let start_s = if let Some(start) = range.0 {
            Some(if self.rand_bool() || self.static_width {
                // static
                format!("{}", start)
            } else {
                // else arbitrary
                let vnum = self.next_unique();
                *self.prior_sets += &format!("let s{} = {};", vnum, start);
                format!("s{}", vnum)
            })
        } else {
            None
        };
        let end_s = if let Some(end) = range.1 {
            Some(if self.rand_bool() || self.static_width {
                // static
                format!("{}", end - inclusive)
            } else {
                // else arbitrary
                let vnum = self.next_unique();
                *self.prior_sets += &format!("let e{} = {};", vnum, end - inclusive);
                format!("e{}", vnum)
            })
        } else {
            None
        };
        (start_s, end_s, inclusive == 1)
    }

    pub fn gen_index(&mut self, range: (Option<usize>, Option<usize>)) -> String {
        let (start_s, end_s, inclusive) = self.gen_range(range);
        match (start_s, end_s) {
            (None, None) => {
                if self.rand_bool() {
                    "".to_owned()
                } else if inclusive {
                    "[..=]".to_owned()
                } else {
                    "[..]".to_owned()
                }
            }
            (None, Some(end)) => {
                if inclusive {
                    if self.rand_bool() {
                        format!("[..={}]", end)
                    } else {
                        format!("[0..={}]", end)
                    }
                } else {
                    if self.rand_bool() {
                        format!("[..{}]", end)
                    } else {
                        format!("[0..{}]", end)
                    }
                }
            }
            (Some(start), None) => {
                if inclusive {
                    format!("[{}..=]", start)
                } else {
                    format!("[{}..]", start)
                }
            }
            (Some(start), Some(end)) => {
                if inclusive {
                    format!("[{}..={}]", start, end)
                } else {
                    format!("[{}..{}]", start, end)
                }
            }
        }
    }

    pub fn gen_literal(
        &mut self,
        bitwidth: usize,
        referenced_bw: usize,
        awi: &Bits,
    ) -> (Option<usize>, Option<usize>) {
        let range = if referenced_bw > bitwidth {
            let diff = referenced_bw - bitwidth;
            let offset = self.rand_usize() % diff;
            if self.rand_bool() || self.static_width {
                (Some(offset), Some(bitwidth + offset))
            } else {
                (Some(diff), None)
            }
        } else {
            if self.rand_bool() || self.static_width {
                (None, Some(bitwidth))
            } else {
                (None, None)
            }
        };
        let index = self.gen_index(range);
        self.push_comp(format!("{:?}{}", awi, index));
        range
    }

    pub fn gen_variable(
        &mut self,
        bitwidth: usize,
        referenced_bw: usize,
        awi: &Bits,
        source: bool,
    ) -> (Option<usize>, Option<usize>, String) {
        let range = if referenced_bw > bitwidth {
            let diff = referenced_bw - bitwidth;
            let offset = self.rand_usize() % diff;
            if self.rand_bool() || self.static_width {
                (Some(offset), Some(bitwidth + offset))
            } else {
                (Some(diff), None)
            }
        } else {
            if self.rand_bool() || self.static_width {
                (None, Some(bitwidth))
            } else {
                (None, None)
            }
        };
        let vnum = self.next_unique();
        let mutability = if source {
            String::new()
        } else {
            "mut ".to_owned()
        };
        let ref_s = match self.rand_usize() % 3 {
            // Bits
            0 => {
                let ref_fn = if source {
                    "ref".to_owned()
                } else {
                    "mut".to_owned()
                };
                *self.prior_sets += &format!(
                    "let {}awi{} = inlawi!({:?});let bits{} = awi{}.const_as_{}();\n",
                    mutability, vnum, awi, vnum, vnum, ref_fn
                );
                format!("bits{}", vnum)
            }
            // InlAwi
            1 => {
                *self.prior_sets +=
                    &format!("let {}inl{} = inlawi!({:?});\n", mutability, vnum, awi);
                format!("inl{}", vnum)
            }
            // ExtAwi
            _ => {
                *self.prior_sets +=
                    &format!("let {}ext{} = extawi!({:?});\n", mutability, vnum, awi);
                format!("ext{}", vnum)
            }
        };
        let index = self.gen_index(range);
        self.push_comp(format!("{}{}", ref_s, index));
        (range.0, range.1, ref_s)
    }

    /// returns a string for the range
    pub fn gen_filler(
        &mut self,
        bitwidth: usize,
        force_bounded_filler: bool,
        comp_i: usize,
    ) -> (usize, Option<usize>) {
        // make unbounded fillers more common
        let modifier = self.rand_usize() % 8;
        let range = if modifier == 7 {
            let tmp = (self.rand_usize() as u8) as usize;
            (Some(tmp), Some(bitwidth + tmp))
        } else if (modifier == 6)
            || self.dynamic_width
            || self.unbounded
            || (self.align.is_ls() && (comp_i != (self.num_comps - 1)))
            || (self.align.is_ms() && (comp_i != 0))
            || force_bounded_filler
        {
            (None, Some(bitwidth))
        } else {
            // unbounded filler
            (None, None)
        };
        let range_s = match self.gen_range(range) {
            (None, None, false) => "..".to_owned(),
            (None, None, true) => "..=".to_owned(),
            (None, Some(end), false) => {
                if self.rand_bool() {
                    format!("..{}", end)
                } else {
                    format!("0..{}", end)
                }
            }
            (None, Some(end), true) => {
                if self.rand_bool() {
                    format!("..={}", end)
                } else {
                    format!("0..={}", end)
                }
            }
            (Some(start), None, false) => format!("{}..", start),
            (Some(start), None, true) => format!("{}..=", start),
            (Some(start), Some(end), false) => format!("{}..{}", start, end),
            (Some(start), Some(end), true) => format!("{}..={}", start, end),
        };
        self.push_comp(range_s);
        (if let Some(start) = range.0 { start } else { 0 }, range.1)
    }

    pub fn gen_source_concat(&mut self) {
        for comp_i in 0..self.num_comps {
            let (bitwidth, referenced_bw) = self.gen_comp_bitwidth(comp_i);
            let mut comp_type = match self.rand_usize() % 8 {
                0..=5 => 2,
                6 => 0,
                // (7)
                _ => 1,
            };
            if !self.specified_initialization || (comp_type == 0) || (comp_type == 1) {
                if comp_type == 2 {
                    // override
                    comp_type = 1;
                }

                let mut awi = ExtAwi::zero(bw(referenced_bw));
                awi[..].rand_assign_using(self.rng).unwrap();

                let mut range;
                // 0 is literal, 1 is variable
                if comp_type == 0 {
                    range = self.gen_literal(bitwidth, referenced_bw, &awi[..]);
                } else {
                    // variable
                    let tmp = self.gen_variable(bitwidth, referenced_bw, &awi[..], true);
                    range = (tmp.0, tmp.1);
                }

                if let Some(ref mut start) = range.0 {
                    let mut tmp = ExtAwi::zero(bw(awi.bw() - *start));
                    awi[..].lshr_assign(*start).unwrap();
                    tmp[..].zero_resize_assign(&awi[..]);
                    awi = tmp;
                    if let Some(ref mut end) = range.1 {
                        *end -= *start;
                    }
                    *start = 0;
                }
                if let Some(end) = range.1 {
                    let mut tmp = ExtAwi::zero(bw(end));
                    tmp[..].zero_resize_assign(&awi[..]);
                    awi = tmp;
                }
                let nzbw = awi.nzbw();
                self.non_unbounded_width += nzbw.get();
                self.push_awi(awi, ExtAwi::zero(nzbw));
            } else {
                // 2 is filler
                let (start, end) = self.gen_filler(bitwidth, self.only_one_concat, comp_i);
                if let Some(end) = end {
                    let awi = ExtAwi::zero(bw(end - start));
                    self.non_unbounded_width += awi.bw();
                    let mask = ExtAwi::umax(awi.nzbw());
                    self.push_awi(awi, mask);
                } else {
                    self.align_side = true;
                    self.unbounded = true;
                }
            }
        }
        self.set_middle_filler_mask();
    }

    pub fn gen_sink_concat(&mut self) {
        self.assertions += "let mut _shl = 0;\n";
        for comp_i in 0..self.num_comps {
            let (bitwidth, referenced_bw) = self.gen_comp_bitwidth(comp_i);
            let b = match self.rand_usize() % 8 {
                0..=5 => false,
                _ => true,
            };
            if b {
                // variable
                let mut awi = ExtAwi::zero(bw(referenced_bw));
                awi[..].rand_assign_using(self.rng).unwrap();

                let tmp = self.gen_variable(bitwidth, referenced_bw, &awi[..], false);
                let start = tmp.0;
                let end = tmp.1;
                let ref_s = tmp.2;

                // calculate resulting value
                let sc = match (start, end) {
                    (None, None) => awi.bw(),
                    (Some(start), None) => awi.bw() - start,
                    (None, Some(end)) => end,
                    (Some(start), Some(end)) => end - start,
                };
                self.non_unbounded_width += sc;
                let start = if let Some(start) = start { start } else { 0 };

                // This is done in this awkward way because I would need to refactor in order to
                // know the actual maximum bitwidth ahead of time for all unbounded cases. TODO
                // this should just be a single assertion with no dynamic `_result` or `shl`
                // assignments.
                if self.align_side {
                    self.assertions += &format!("_shl -= {};\n", sc);
                }
                self.assertions += &format!(
                    "let mut _result = inlawi!({:?});\n_result[..].field({}, &_source[..], _shl, \
                     {}).unwrap();\nassert_eq!({}.const_as_ref(), &_result[..]);\n",
                    awi, start, sc, ref_s
                );
                if !self.align_side {
                    self.assertions += &format!("_shl += {};\n", sc);
                }
            } else {
                // filler
                let (start, end) = self.gen_filler(bitwidth, self.num_comps == 1, comp_i);
                if let Some(end) = end {
                    let sc = end - start;
                    self.non_unbounded_width += sc;
                    if self.align_side {
                        self.assertions += &format!("_shl -= {};\n", sc);
                    } else {
                        self.assertions += &format!("_shl += {};\n", sc);
                    }
                } else {
                    self.assertions += "let mut _shl = _source.bw();";
                    self.align_side = true;
                    self.unbounded = true;
                }
            }
        }
    }
}

fn main() {
    // remove prior `generated.rs`
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let out_file = out_dir.join("generated.rs");
    drop(fs::remove_file(&out_file));

    let mut s = "#[test] fn generated_macro_fuzz_test() {\n".to_owned();
    let mut rng = &mut Xoshiro128StarStar::seed_from_u64(0);
    let mut vnum = 0;
    // number of tests generated
    for test_i in 0..NUM_TESTS {
        s += &format!("// {}\n", test_i);
        // make unbounded source fillers more common, where more edge cases are
        let awi_type = match rng.next_u32() % 8 {
            0..=5 => 0,
            6 => 1,
            // (7)
            _ => 2,
        };
        let is_cc = awi_type == 0;
        let is_inlawi = awi_type == 1;
        let is_extawi = awi_type == 2;
        let specified_initialization = (rng.next_u32() & 1) == 0;

        // note: this is the suggested width, in the all unbounded case it may be less
        let mut width = ((rng.next_u32() as usize) % MAX_BW) + MAX_COMPS + 1;
        let mut nzbw = bw(width);
        let num_concats = ((rng.next_u32() as usize) % MAX_CONCATS) + 1;

        // specifies which concatenation needs to have a static or dynamic width. If set
        // to `num_concats`, no concatenation is required to have static or dynamic
        // width.
        let static_width_i = if is_inlawi {
            (rng.next_u32() as usize) % num_concats
        } else {
            num_concats
        };
        let align = match rng.next_u32() % 3 {
            0 => Align::Ls,
            1 => Align::Ms,
            _ => Align::Any,
        };
        let dynamic_width_i = if ((static_width_i != num_concats) && is_extawi) || align.is_any() {
            (rng.next_u32() as usize) % num_concats
        } else {
            num_concats
        };

        // source concatenation
        let mut prior_sets = String::new();
        let mut c = Concat::new(
            nzbw,
            &mut rng,
            &mut vnum,
            &mut prior_sets,
            dynamic_width_i == 0,
            static_width_i == 0,
            align,
            specified_initialization,
            num_concats == 1,
        );
        c.gen_source_concat();
        // used to handle general unbounded cases
        let mut max_concat_width = c.non_unbounded_width;
        let mut source_val = c.val.clone();
        let mut source_fill = c.fill.clone();
        let mut concats = c.get_concat_s();
        drop(c);

        // sink concatenations
        let mut assertions = String::new();
        for concat_i in 1..num_concats {
            concats += ";\n";
            let mut c = Concat::new(
                nzbw,
                &mut rng,
                &mut vnum,
                &mut prior_sets,
                dynamic_width_i == concat_i,
                static_width_i == concat_i,
                align,
                specified_initialization,
                false,
            );
            c.gen_sink_concat();
            max_concat_width = max(c.non_unbounded_width, max_concat_width);
            assertions += &c.assertions;
            concats += &c.get_concat_s();
        }

        if width != max_concat_width {
            // all unbounded cases
            let diff = width - max_concat_width;
            nzbw = bw(max_concat_width);
            let mut tmp = ExtAwi::zero(nzbw);
            match align {
                Align::Ls => {
                    tmp[..].zero_resize_assign(&source_fill[..]);
                    source_fill = tmp.clone();
                    tmp[..].zero_resize_assign(&source_val[..]);
                    source_val = tmp.clone();
                }
                Align::Ms => {
                    source_fill[..].lshr_assign(diff).unwrap();
                    tmp[..].zero_resize_assign(&source_fill[..]);
                    source_fill = tmp.clone();
                    source_val[..].lshr_assign(diff).unwrap();
                    tmp[..].zero_resize_assign(&source_val[..]);
                    source_val = tmp.clone();
                }
                Align::Any => unreachable!(),
            }
            width = max_concat_width;
        }
        let (mut source, construct_fn) = match rng.next_u32() % 5 {
            0 => (ExtAwi::zero(nzbw), "zero".to_owned()),
            1 => (ExtAwi::umax(nzbw), "umax".to_owned()),
            2 => (ExtAwi::imax(nzbw), "imax".to_owned()),
            3 => (ExtAwi::imin(nzbw), "imin".to_owned()),
            _ => (ExtAwi::uone(nzbw), "uone".to_owned()),
        };
        source[..].and_assign(&source_fill[..]).unwrap();
        source[..].or_assign(&source_val[..]).unwrap();
        let (macro_root, eq_fn) = if is_cc {
            ("cc".to_owned(), "eq_unit".to_owned())
        } else if is_inlawi {
            (
                "inlawi".to_owned(),
                format!(
                    "eq_inl::<{}, {}>",
                    width,
                    width
                        .wrapping_shr(usize::MAX.count_ones().trailing_zeros())
                        .wrapping_add(
                            (width & ((usize::MAX.count_ones() as usize) - 1) != 0) as usize
                        )
                        .wrapping_add(1)
                ),
            )
        } else {
            ("extawi".to_owned(), "eq_ext".to_owned())
        };
        let macro_suffix = if specified_initialization {
            format!("_{}", construct_fn)
        } else {
            String::new()
        };

        let eq_rhs = if is_cc {
            "()".to_owned()
        } else {
            format!("{}!({:?})", macro_root, source)
        };

        s += &format!(
            "let _source = inlawi!({:?});\n{}{}(&{}{}!({}),\n{}\n);\n{}\n",
            source, prior_sets, eq_fn, macro_root, macro_suffix, concats, eq_rhs, assertions
        );
    }
    s += "}";

    OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(out_file)
        .unwrap()
        .write_all(s.as_bytes())
        .unwrap();
}
