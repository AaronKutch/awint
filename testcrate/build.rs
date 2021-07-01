// Used by `tests/macro_fuzzing.rs`.
// Here, we try to generate code which tests all successful code generation
// paths

use std::{cmp::min, env, fs, fs::OpenOptions, io::Write, num::NonZeroUsize, path::PathBuf};

use awint::prelude::*;
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};

// number of tests generated
const NUM_TESTS: usize = 500;
// should be plenty to test all edge cases
const MAX_CONCATS: usize = 5;
// enough to get multiple components on each side of an unbounded filler
const MAX_COMPS: usize = 5;
// max total bitwidth
const MAX_BW: usize = 7 * (MAX_COMPS as usize);

#[derive(Debug)]
struct Concat {
    /// The value. Filler bits are zeroed
    pub val: ExtAwi,
    /// Filler mask
    pub fill: ExtAwi,
    /// Alignment side, `false` is least significant
    pub align: bool,
    /// Least significant shift position
    pub ls_shift: usize,
    /// More significant shift position
    pub ms_shift: usize,
}

impl Concat {
    /// Note: this starts with `align == true`
    pub fn new(bw: NonZeroUsize) -> Self {
        Self {
            val: ExtAwi::zero(bw),
            fill: ExtAwi::zero(bw),
            align: true,
            ls_shift: 0,
            ms_shift: bw.get(),
        }
    }

    pub fn append_awi(&mut self, val: ExtAwi, fill: ExtAwi) {
        assert_eq!(val.bw(), fill.bw());
        if self.align {
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
}

fn gen_concat(
    rng: &mut Xoshiro128StarStar,
    vnum: &mut u64,
    specified_initialization: bool,
    static_width: bool,
    // note: this is automatically set to true if `static_width` is true
    dynamic_width: bool,
    // note: this is a recommendation, `dynamic_width` overrides this
    unbounded_alignment: u32,
    width: usize,
) -> (String, Concat, String, bool) {
    let mut dynamic_width = dynamic_width;
    if static_width {
        dynamic_width = true;
    }
    let mut external = String::new();
    let mut c = Concat::new(bw(width));
    let mut input = String::new();
    let num_comps = min(
        (((rng.next_u32() as usize) % MAX_COMPS) + 1) as usize,
        width,
    );
    let mut remaining_width = width;
    let mut unbounded = false;
    let mut fallible = false;
    for comp_i in 0..num_comps {
        // we have to use up the total `width`. This also makes sure there are enough
        // single bits left
        let bitwidth = if comp_i == (num_comps - 1) {
            remaining_width
        } else {
            // make sure there is at least a bit for every component remaining
            let limiter = remaining_width - (num_comps - comp_i);
            (((rng.next_u32() as usize) % ((2 * remaining_width) / (num_comps - comp_i))) % limiter)
                + 1
        };
        let referenced_bw = if (rng.next_u32() & 1) == 0 {
            // full range
            bitwidth
        } else {
            // make the bitwidth of the referenced component larger
            bitwidth + ((rng.next_u32() as usize) % bitwidth)
        };
        remaining_width -= bitwidth;
        let mut comp_type = rng.next_u32() % 3;
        if !specified_initialization || (comp_type == 0) || (comp_type == 1) {
            if comp_type == 2 {
                comp_type = 1;
            }

            let mut awi = ExtAwi::zero(bw(referenced_bw));
            awi[..].rand_assign_using(rng).unwrap();

            let inclusive = ((rng.next_u32() % 4) == 0) as usize;
            let b = (rng.next_u32() & 1) == 0;
            let range = if referenced_bw > bitwidth {
                let diff = referenced_bw - bitwidth;
                let offset = (rng.next_u32() as usize) % diff;
                if b || static_width {
                    (Some(offset), Some(bitwidth + offset))
                } else {
                    (Some(diff), None)
                }
            } else {
                if b || static_width {
                    (None, Some(bitwidth))
                } else {
                    (None, None)
                }
            };
            let start_s = if let Some(start) = range.0 {
                let b = (rng.next_u32() & 1) == 0;
                Some(if b || static_width {
                    // static
                    format!("{}", start)
                } else {
                    // else arbitrary
                    fallible = true;
                    external += &format!("let s{} = {};", vnum, start);
                    let s = format!("s{}", vnum);
                    *vnum += 1;
                    s
                })
            } else {
                None
            };
            let end_s = if let Some(end) = range.1 {
                let b = (rng.next_u32() & 1) == 0;
                Some(if b || static_width {
                    // static
                    format!("{}", end - inclusive)
                } else {
                    // else arbitrary
                    fallible = true;
                    external += &format!("let e{} = {};", vnum, end - inclusive);
                    let s = format!("e{}", vnum);
                    *vnum += 1;
                    s
                })
            } else {
                None
            };
            let b = (rng.next_u32() & 1) == 0;
            if (start_s.is_some() || end_s.is_some()) && (comp_type == 1) {
                fallible = true;
            }
            let range_s = match (start_s, end_s) {
                (None, None) => {
                    if b {
                        "".to_owned()
                    } else if inclusive == 1 {
                        "[..=]".to_owned()
                    } else {
                        "[..]".to_owned()
                    }
                }
                (None, Some(end)) => {
                    if inclusive == 1 {
                        if b {
                            format!("[..={}]", end)
                        } else {
                            format!("[0..={}]", end)
                        }
                    } else {
                        if b {
                            format!("[..{}]", end)
                        } else {
                            format!("[0..{}]", end)
                        }
                    }
                }
                (Some(start), None) => {
                    if inclusive == 1 {
                        format!("[{}..=]", start)
                    } else {
                        format!("[{}..]", start)
                    }
                }
                (Some(start), Some(end)) => {
                    if inclusive == 1 {
                        format!("[{}..={}]", start, end)
                    } else {
                        format!("[{}..{}]", start, end)
                    }
                }
            };

            // 0 is literal, 1 is variable
            if comp_type == 0 {
                // literal
                input += &format!("{:?}{}, ", awi, range_s);
            } else {
                // variable
                let ref_s = match rng.next_u32() % 3 {
                    // Bits
                    0 => {
                        external += &format!(
                            "let awi{} = inlawi!({:?});let bits{} = awi{}.const_as_ref();\n",
                            vnum, awi, vnum, vnum
                        );
                        format!("bits{}", vnum)
                    }
                    // InlAwi
                    1 => {
                        external += &format!("let inl{} = inlawi!({:?});\n", vnum, awi);
                        format!("inl{}", vnum)
                    }
                    // ExtAwi
                    _ => {
                        external += &format!("let ext{} = extawi!({:?});\n", vnum, awi);
                        format!("ext{}", vnum)
                    }
                };
                input += &format!("{}{}, ", ref_s, range_s);
                *vnum += 1;
            }

            // truncate and append to calculated value
            let mut range = range;
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
            c.append_awi(awi, ExtAwi::zero(nzbw));
        } else {
            // 2 is filler
            let b = (rng.next_u32() & 1) == 0;
            let range = if b {
                let tmp = (rng.next_u32() as u8) as usize;
                (Some(tmp), Some(bitwidth + tmp))
            } else {
                let b = (rng.next_u32() & 1) == 0;
                if b || dynamic_width
                    || unbounded
                    || ((unbounded_alignment == 1) && (comp_i != (num_comps - 1)))
                    || ((unbounded_alignment == 2) && (comp_i != 0))
                {
                    (None, Some(bitwidth))
                } else {
                    // unbounded filler
                    (None, None)
                }
            };
            // only fallible if both sides of range exist and at least one is arbitrary
            let possible_fallible = range.0.is_some() && range.1.is_some();
            let start_s = if let Some(start) = range.0 {
                let b = (rng.next_u32() & 1) == 0;
                Some(if b || static_width {
                    format!("{}", start)
                } else {
                    // else arbitrary
                    if possible_fallible {
                        fallible = true;
                    }
                    external += &format!("let s{} = {};", vnum, start);
                    let s = format!("s{}", vnum);
                    *vnum += 1;
                    s
                })
            } else {
                None
            };
            let end_s = if let Some(end) = range.1 {
                let b = (rng.next_u32() & 1) == 0;
                Some(if b || static_width {
                    format!("{}", end)
                } else {
                    // else arbitrary
                    if possible_fallible {
                        fallible = true;
                    }
                    external += &format!("let e{} = {};", vnum, end);
                    let s = format!("e{}", vnum);
                    *vnum += 1;
                    s
                })
            } else {
                None
            };
            let b = (rng.next_u32() & 1) == 0;
            let range_s = match (start_s, end_s) {
                (None, None) => "..".to_owned(),
                (None, Some(end)) => {
                    if b {
                        format!("..{}", end)
                    } else {
                        format!("0..{}", end)
                    }
                }
                (Some(start), None) => format!("{}..", start),
                (Some(start), Some(end)) => format!("{}..{}", start, end),
            };

            input += &format!("{}, ", range_s);
            let (start, end) = (if let Some(start) = range.0 { start } else { 0 }, range.1);
            if let Some(end) = end {
                let awi = ExtAwi::zero(bw(end - start));
                let mask = ExtAwi::umax(awi.nzbw());
                c.append_awi(awi, mask);
            } else {
                c.align = false;
                unbounded = true;
            }
        }
    }
    c.set_middle_filler_mask();
    (external, c, input, fallible)
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
    for _ in 0..NUM_TESTS {
        let awi_type = rng.next_u32() % 3;
        let is_cc = awi_type == 0;
        let is_inlawi = awi_type == 1;
        let is_extawi = awi_type == 2;
        let specified_initialization = (rng.next_u32() & 1) == 0;

        let width = ((rng.next_u32() as usize) % MAX_BW) + MAX_COMPS + 1;
        let nzbw = bw(width);
        let num_concats = 1; //((rng.next_u32() as usize) % MAX_CONCATS) + 1;

        // specifies which concatenation needs to have a static or dynamic width. If set
        // to `num_concats`, no concatenation is required to have static or dynamic
        // width.
        let static_width_i = if is_inlawi {
            (rng.next_u32() as usize) % num_concats
        } else {
            num_concats
        };
        // 0 is any, 1 is ls, 2 is ms
        let unbounded_alignment = rng.next_u32() % 3;
        let dynamic_width_i = if is_extawi || (unbounded_alignment == 0) || (num_concats == 1) {
            (rng.next_u32() as usize) % num_concats
        } else {
            num_concats
        };

        // generate source concatenation
        let (source_ext, source_c, source_input, fallible) = gen_concat(
            &mut rng,
            &mut vnum,
            specified_initialization,
            static_width_i == 0,
            dynamic_width_i == 0,
            unbounded_alignment,
            width,
        );

        let (mut source, construct_fn) = match rng.next_u32() % 5 {
            0 => (ExtAwi::zero(nzbw), "zero".to_owned()),
            1 => (ExtAwi::umax(nzbw), "umax".to_owned()),
            2 => (ExtAwi::imax(nzbw), "imax".to_owned()),
            3 => (ExtAwi::imin(nzbw), "imin".to_owned()),
            _ => (ExtAwi::uone(nzbw), "uone".to_owned()),
        };
        source[..].and_assign(&source_c.fill[..]);
        source[..].or_assign(&source_c.val[..]);

        let macro_root = if is_cc {
            "cc".to_owned()
        } else if is_inlawi {
            "inlawi".to_owned()
        } else {
            "extawi".to_owned()
        };
        let macro_suffix = if specified_initialization {
            format!("_{}", construct_fn)
        } else {
            String::new()
        };

        let mut eq_rhs = if is_cc {
            "()".to_owned()
        } else {
            format!("{}!({:?})", macro_root, source)
        };
        if fallible {
            eq_rhs = format!("Some({})", eq_rhs);
        }

        s += &format!(
            "{}assert_eq!({}{}!({}), {});\n\n",
            source_ext, macro_root, macro_suffix, source_input, eq_rhs
        );
        //dbg!(source_ext, source_c, source_input);
        //panic!();

        // sink concatenations
        //for concat_i in 0..num_concats {

        //}
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
