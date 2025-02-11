use std::{
    cmp::{max, min},
    num::NonZeroUsize,
};

use awint::{
    awi,
    awint_internals::USIZE_BITS,
    awint_macro_internals::triple_arena::{ptr_struct, Arena},
    dag,
};
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};

use crate::dag_tests::{Epoch, EvalAwi, LazyAwi};

// miri is just here to check that the unsized deref hacks are working
const N: (u32, u32) = if cfg!(miri) {
    (1, 1)
} else if cfg!(debug_assertions) {
    (32, 100)
} else {
    (32, 1000)
};

ptr_struct!(P0);

#[derive(Debug)]
struct Pair {
    awi: awi::Awi,
    dag: dag::Awi,
    eval: Option<EvalAwi>,
}

#[derive(Debug)]
struct Mem {
    a: Arena<P0, Pair>,
    // `LazyAwi`s that get need to be retro assigned
    roots: Vec<(LazyAwi, awi::Awi)>,
    // The outer Vec has `v_len` Vecs for all the supported bitwidths (there is a dummy 0
    // bitwidth Vec and one for each of 1..=(v_len - 1)), the inner Vecs are unsorted and used for
    // random querying
    v: Vec<Vec<P0>>,
    v_len: usize,
    rng: Xoshiro128StarStar,
}

impl Mem {
    pub fn new() -> Self {
        let mut v = vec![];
        let v_len = max(65, USIZE_BITS + 1);
        for _ in 0..v_len {
            v.push(vec![]);
        }
        Self {
            a: Arena::<P0, Pair>::new(),
            roots: vec![],
            v,
            v_len,
            rng: Xoshiro128StarStar::seed_from_u64(0),
        }
    }

    pub fn clear(&mut self) {
        self.a.clear();
        self.v.clear();
        self.roots.clear();
        for _ in 0..self.v_len {
            self.v.push(vec![]);
        }
    }

    /// Randomly creates a new pair or gets an existing one under the `cap`
    pub fn next_capped(&mut self, w: usize, cap: usize) -> P0 {
        let try_query = (self.rng.next_u32() % 4) != 0;
        if try_query && (!self.v[w].is_empty()) {
            let p = self.v[w][(self.rng.next_u32() as usize) % self.v[w].len()];
            if self.get_awi(p).to_usize() < cap {
                return p
            }
        }
        let nzbw = NonZeroUsize::new(w).unwrap();
        let lazy = LazyAwi::opaque(nzbw);
        let mut lit = awi::Awi::zero(nzbw);
        lit.rand_(&mut self.rng);
        let tmp = lit.to_usize() % cap;
        lit.usize_(tmp);
        let p = self.a.insert(Pair {
            awi: lit.clone(),
            dag: dag::Awi::from(lazy.as_ref()),
            eval: None,
        });
        self.roots.push((lazy, lit));
        self.v[w].push(p);
        p
    }

    /// Randomly creates a new pair or gets an existing one
    pub fn next(&mut self, w: usize) -> P0 {
        let try_query = (self.rng.next_u32() % 4) != 0;
        if try_query && (!self.v[w].is_empty()) {
            self.v[w][(self.rng.next_u32() as usize) % self.v[w].len()]
        } else {
            let nzbw = NonZeroUsize::new(w).unwrap();
            let lazy = LazyAwi::opaque(nzbw);
            let mut lit = awi::Awi::zero(nzbw);
            lit.rand_(&mut self.rng);
            let p = self.a.insert(Pair {
                awi: lit.clone(),
                dag: dag::Awi::from(lazy.as_ref()),
                eval: None,
            });
            self.roots.push((lazy, lit));
            self.v[w].push(p);
            p
        }
    }

    /// Calls `next` with a random integer in 1..5, returning a tuple of the
    /// width chosen and the Ptr to what `next` returned.
    pub fn next4(&mut self) -> (usize, P0) {
        let w = ((self.rng.next_u32() as usize) % 4) + 1;
        (w, self.next(w))
    }

    pub fn next_usize(&mut self, cap: usize) -> P0 {
        self.next_capped(USIZE_BITS, cap)
    }

    // just use cloning for the immutable indexing, because dealing with the guards
    // of mixed internal mutability is too much. We can't get the signature of
    // `Index` to work in any case.

    pub fn get_awi(&self, inx: P0) -> awi::Awi {
        self.a[inx].awi.clone()
    }

    pub fn get_dag(&self, inx: P0) -> dag::Awi {
        self.a[inx].dag.clone()
    }

    pub fn get_mut_awi(&mut self, inx: P0) -> &mut awi::Awi {
        &mut self.a[inx].awi
    }

    pub fn get_mut_dag(&mut self, inx: P0) -> &mut dag::Awi {
        &mut self.a[inx].dag
    }

    pub fn finish(&mut self, _epoch: &Epoch) {
        for pair in self.a.vals_mut() {
            pair.eval = Some(EvalAwi::from(&pair.dag))
        }
    }

    pub fn eval_and_verify_equal(&mut self, _epoch: &Epoch) {
        // set all lazy roots
        for (lazy, lit) in &mut self.roots {
            lazy.retro_(lit).unwrap();
        }
        // evaluate all
        for pair in self.a.vals() {
            assert_eq!(pair.eval.as_ref().unwrap().eval().unwrap(), pair.awi);
        }
    }
}

fn num_dag_duo(rng: &mut Xoshiro128StarStar, m: &mut Mem) {
    let next_op = rng.next_u32() % 31;
    match next_op {
        // Lut, StaticLut
        0 => {
            let (out_w, out) = m.next4();
            let (inx_w, inx) = m.next4();
            let lut = m.next(out_w * (1 << inx_w));
            let lut_a = m.get_awi(lut);
            let inx_a = m.get_awi(inx);
            m.get_mut_awi(out).lut_(&lut_a, &inx_a).unwrap();
            let lut_b = m.get_dag(lut);
            let inx_b = m.get_dag(inx);
            m.get_mut_dag(out).lut_(&lut_b, &inx_b).unwrap();
        }
        // Get, StaticGet
        1 => {
            let (bits_w, bits) = m.next4();
            let inx = m.next_usize(bits_w);
            let out = m.next(1);
            let bits_a = m.get_awi(bits);
            let inx_a = m.get_awi(inx);
            m.get_mut_awi(out)
                .bool_(bits_a.get(inx_a.to_usize()).unwrap());
            let bits_b = m.get_dag(bits);
            let inx_b = m.get_dag(inx);
            m.get_mut_dag(out)
                .bool_(bits_b.get(inx_b.to_usize()).unwrap());
        }
        // Set, StaticSet
        2 => {
            let (bits_w, bits) = m.next4();
            let inx = m.next_usize(bits_w);
            let bit = m.next(1);
            let inx_a = m.get_awi(inx);
            let bit_a = m.get_awi(bit);
            m.get_mut_awi(bits)
                .set(inx_a.to_usize(), bit_a.to_bool())
                .unwrap();
            let inx_b = m.get_dag(inx);
            let bit_b = m.get_dag(bit);
            m.get_mut_dag(bits)
                .set(inx_b.to_usize(), bit_b.to_bool())
                .unwrap();
        }
        // FieldBit
        3 => {
            let (lhs_w, lhs) = m.next4();
            let to = m.next_usize(lhs_w);
            let (rhs_w, rhs) = m.next4();
            let from = m.next_usize(rhs_w);
            let to_a = m.get_awi(to);
            let rhs_a = m.get_awi(rhs);
            let from_a = m.get_awi(from);
            m.get_mut_awi(lhs)
                .field_bit(to_a.to_usize(), &rhs_a, from_a.to_usize())
                .unwrap();
            let to_b = m.get_dag(to);
            let rhs_b = m.get_dag(rhs);
            let from_b = m.get_dag(from);
            m.get_mut_dag(lhs)
                .field_bit(to_b.to_usize(), &rhs_b, from_b.to_usize())
                .unwrap();
        }
        // ZeroResize
        4 => {
            let lhs = m.next4().1;
            let rhs = m.next4().1;
            let rhs_a = m.get_awi(rhs);
            m.get_mut_awi(lhs).zero_resize_(&rhs_a);
            let rhs_b = m.get_dag(rhs);
            m.get_mut_dag(lhs).zero_resize_(&rhs_b);
        }
        // SignResize
        5 => {
            let lhs = m.next4().1;
            let rhs = m.next4().1;
            let rhs_a = m.get_awi(rhs);
            m.get_mut_awi(lhs).sign_resize_(&rhs_a);
            let rhs_b = m.get_dag(rhs);
            m.get_mut_dag(lhs).sign_resize_(&rhs_b);
        }
        // Not
        6 => {
            let x = m.next4().1;
            m.get_mut_awi(x).not_();
            m.get_mut_dag(x).not_();
        }
        // Or, And, Xor
        7 => {
            let (lhs_w, lhs) = m.next4();
            let rhs = m.next(lhs_w);
            let rhs_a = m.get_awi(rhs);
            let rhs_b = m.get_dag(rhs);
            match rng.next_u32() % 3 {
                0 => {
                    m.get_mut_awi(lhs).or_(&rhs_a).unwrap();
                    m.get_mut_dag(lhs).or_(&rhs_b).unwrap();
                }
                1 => {
                    m.get_mut_awi(lhs).and_(&rhs_a).unwrap();
                    m.get_mut_dag(lhs).and_(&rhs_b).unwrap();
                }
                2 => {
                    m.get_mut_awi(lhs).xor_(&rhs_a).unwrap();
                    m.get_mut_dag(lhs).xor_(&rhs_b).unwrap();
                }
                _ => unreachable!(),
            }
        }
        // Inc, IncCout, Dec, DecCout
        8 => {
            let x = m.next4().1;
            let cin = m.next(1);
            let cout = m.next(1);
            let cin_a = m.get_awi(cin);
            let cin_b = m.get_dag(cin);
            let out_a;
            let out_b;
            if (rng.next_u32() & 1) == 0 {
                out_a = m.get_mut_awi(x).inc_(cin_a.to_bool());
                out_b = m.get_mut_dag(x).inc_(cin_b.to_bool());
            } else {
                out_a = m.get_mut_awi(x).dec_(cin_a.to_bool());
                out_b = m.get_mut_dag(x).dec_(cin_b.to_bool());
            }
            m.get_mut_awi(cout).bool_(out_a);
            m.get_mut_dag(cout).bool_(out_b);
        }
        // CinSum, UnsignedOverflow, SignedOverflow
        9 => {
            let cin = m.next(1);
            let (lhs_w, lhs) = m.next4();
            let rhs = m.next(lhs_w);
            let out = m.next(lhs_w);
            let unsigned = m.next(1);
            let signed = m.next(1);

            let cin_a = m.get_awi(cin);
            let lhs_a = m.get_awi(lhs);
            let rhs_a = m.get_awi(rhs);
            let overflow = m
                .get_mut_awi(out)
                .cin_sum_(cin_a.to_bool(), &lhs_a, &rhs_a)
                .unwrap();
            m.get_mut_awi(unsigned).bool_(overflow.0);
            m.get_mut_awi(signed).bool_(overflow.1);

            let cin_b = m.get_dag(cin);
            let lhs_b = m.get_dag(lhs);
            let rhs_b = m.get_dag(rhs);
            let overflow = m
                .get_mut_dag(out)
                .cin_sum_(cin_b.to_bool(), &lhs_b, &rhs_b)
                .unwrap();
            m.get_mut_dag(unsigned).bool_(overflow.0);
            m.get_mut_dag(signed).bool_(overflow.1);
        }
        // Lsb, Msb
        10 => {
            let x = m.next4().1;
            let out = m.next(1);
            if (rng.next_u32() & 1) == 0 {
                let a = m.get_awi(x).lsb();
                m.get_mut_awi(out).bool_(a);
                let b = m.get_dag(x).lsb();
                m.get_mut_dag(out).bool_(b);
            } else {
                let a = m.get_awi(x).msb();
                m.get_mut_awi(out).bool_(a);
                let b = m.get_dag(x).msb();
                m.get_mut_dag(out).bool_(b);
            }
        }
        // Neg, Abs
        11 => {
            let x = m.next4().1;
            if (rng.next_u32() & 1) == 0 {
                let neg = m.next(1);
                let a = m.get_awi(neg).to_bool();
                m.get_mut_awi(x).neg_(a);
                let b = m.get_dag(neg).to_bool();
                m.get_mut_dag(x).neg_(b);
            } else {
                m.get_mut_awi(x).abs_();
                m.get_mut_dag(x).abs_();
            }
        }
        // Funnel
        12 => {
            let w = 1 << (((m.rng.next_u32() as usize) % 2) + 1);
            let lhs = m.next(w);
            let rhs = m.next(w * 2);
            let s = m.next(w.trailing_zeros() as usize);
            let a = m.get_awi(rhs);
            let a_s = m.get_awi(s);
            m.get_mut_awi(lhs).funnel_(&a, &a_s).unwrap();
            let b = m.get_dag(rhs);
            let b_s = m.get_dag(s);
            m.get_mut_dag(lhs).funnel_(&b, &b_s).unwrap();
        }
        // RangeOr, RangeAnd, RangeXor
        13 => {
            let (w0, x) = m.next4();
            let start = m.next_usize(w0 + 1);
            let end = m.next_usize(w0 + 1);
            let start_a = m.get_awi(start);
            let end_a = m.get_awi(end);
            let start_b = m.get_dag(start);
            let end_b = m.get_dag(end);
            match rng.next_u32() % 3 {
                0 => {
                    m.get_mut_awi(x)
                        .range_or_(start_a.to_usize()..end_a.to_usize())
                        .unwrap();
                    m.get_mut_dag(x)
                        .range_or_(start_b.to_usize()..end_b.to_usize())
                        .unwrap();
                }
                1 => {
                    m.get_mut_awi(x)
                        .range_and_(start_a.to_usize()..end_a.to_usize())
                        .unwrap();
                    m.get_mut_dag(x)
                        .range_and_(start_b.to_usize()..end_b.to_usize())
                        .unwrap();
                }
                2 => {
                    m.get_mut_awi(x)
                        .range_xor_(start_a.to_usize()..end_a.to_usize())
                        .unwrap();
                    m.get_mut_dag(x)
                        .range_xor_(start_b.to_usize()..end_b.to_usize())
                        .unwrap();
                }
                _ => unreachable!(),
            }
        }
        // FieldWidth
        14 => {
            let (w0, lhs) = m.next4();
            let (w1, rhs) = m.next4();
            let min_w = min(w0, w1);
            let width = m.next_usize(min_w + 1);
            let rhs_a = m.get_awi(rhs);
            let width_a = m.get_awi(width);
            m.get_mut_awi(lhs)
                .field_width(&rhs_a, width_a.to_usize())
                .unwrap();
            let rhs_b = m.get_dag(rhs);
            let width_b = m.get_dag(width);
            m.get_mut_dag(lhs)
                .field_width(&rhs_b, width_b.to_usize())
                .unwrap();
        }
        // FieldFrom
        15 => {
            let (w0, lhs) = m.next4();
            let (w1, rhs) = m.next4();
            let min_w = min(w0, w1);
            let width = m.next_usize(min_w + 1);
            let from = m.next_usize(1 + w1 - m.get_awi(width).to_usize());
            let rhs_a = m.get_awi(rhs);
            let width_a = m.get_awi(width);
            let from_a = m.get_awi(from);
            m.get_mut_awi(lhs)
                .field_from(&rhs_a, from_a.to_usize(), width_a.to_usize())
                .unwrap();
            let rhs_b = m.get_dag(rhs);
            let width_b = m.get_dag(width);
            let from_b = m.get_dag(from);
            m.get_mut_dag(lhs)
                .field_from(&rhs_b, from_b.to_usize(), width_b.to_usize())
                .unwrap();
        }
        // Shl, Lshr, Ashr, Rotl, Rotr
        16 => {
            let (w, x) = m.next4();
            let s = m.next_usize(w);
            let s_a = m.get_awi(s);
            let s_b = m.get_dag(s);
            match rng.next_u32() % 5 {
                0 => {
                    m.get_mut_awi(x).shl_(s_a.to_usize()).unwrap();
                    m.get_mut_dag(x).shl_(s_b.to_usize()).unwrap();
                }
                1 => {
                    m.get_mut_awi(x).lshr_(s_a.to_usize()).unwrap();
                    m.get_mut_dag(x).lshr_(s_b.to_usize()).unwrap();
                }
                2 => {
                    m.get_mut_awi(x).ashr_(s_a.to_usize()).unwrap();
                    m.get_mut_dag(x).ashr_(s_b.to_usize()).unwrap();
                }
                3 => {
                    m.get_mut_awi(x).rotl_(s_a.to_usize()).unwrap();
                    m.get_mut_dag(x).rotl_(s_b.to_usize()).unwrap();
                }
                4 => {
                    m.get_mut_awi(x).rotr_(s_a.to_usize()).unwrap();
                    m.get_mut_dag(x).rotr_(s_b.to_usize()).unwrap();
                }
                _ => unreachable!(),
            }
        }
        // FieldTo
        17 => {
            let (w0, lhs) = m.next4();
            let (w1, rhs) = m.next4();
            let min_w = min(w0, w1);
            let width = m.next_usize(min_w + 1);
            let to = m.next_usize(1 + w0 - m.get_awi(width).to_usize());
            let to_a = m.get_awi(to);
            let rhs_a = m.get_awi(rhs);
            let width_a = m.get_awi(width);
            m.get_mut_awi(lhs)
                .field_to(to_a.to_usize(), &rhs_a, width_a.to_usize())
                .unwrap();
            let to_b = m.get_dag(to);
            let rhs_b = m.get_dag(rhs);
            let width_b = m.get_dag(width);
            m.get_mut_dag(lhs)
                .field_to(to_b.to_usize(), &rhs_b, width_b.to_usize())
                .unwrap();
        }
        // Add, Sub, Rsb
        18 => {
            let (w, lhs) = m.next4();
            let rhs = m.next(w);
            let rhs_a = m.get_awi(rhs);
            let rhs_b = m.get_dag(rhs);
            match rng.next_u32() % 3 {
                0 => {
                    m.get_mut_awi(lhs).add_(&rhs_a).unwrap();
                    m.get_mut_dag(lhs).add_(&rhs_b).unwrap();
                }
                1 => {
                    m.get_mut_awi(lhs).sub_(&rhs_a).unwrap();
                    m.get_mut_dag(lhs).sub_(&rhs_b).unwrap();
                }
                2 => {
                    m.get_mut_awi(lhs).rsb_(&rhs_a).unwrap();
                    m.get_mut_dag(lhs).rsb_(&rhs_b).unwrap();
                }
                _ => unreachable!(),
            }
        }
        // Field
        19 => {
            let (w0, lhs) = m.next4();
            let (w1, rhs) = m.next4();
            let min_w = min(w0, w1);
            let width = m.next_usize(min_w + 1);
            let to = m.next_usize(1 + w0 - m.get_awi(width).to_usize());
            let from = m.next_usize(1 + w1 - m.get_awi(width).to_usize());

            let to_a = m.get_awi(to);
            let rhs_a = m.get_awi(rhs);
            let from_a = m.get_awi(from);
            let width_a = m.get_awi(width);
            m.get_mut_awi(lhs)
                .field(
                    to_a.to_usize(),
                    &rhs_a,
                    from_a.to_usize(),
                    width_a.to_usize(),
                )
                .unwrap();
            let to_b = m.get_dag(to);
            let rhs_b = m.get_dag(rhs);
            let from_b = m.get_dag(from);
            let width_b = m.get_dag(width);
            m.get_mut_dag(lhs)
                .field(
                    to_b.to_usize(),
                    &rhs_b,
                    from_b.to_usize(),
                    width_b.to_usize(),
                )
                .unwrap();
        }
        // Rev
        20 => {
            let x = m.next4().1;
            m.get_mut_awi(x).rev_();
            m.get_mut_dag(x).rev_();
        }
        // Eq, Ne, Ult, Ule, Ilt, Ile
        21 => {
            let (w, lhs) = m.next4();
            let rhs = m.next(w);
            let lhs_a = m.get_awi(lhs);
            let lhs_b = m.get_dag(lhs);
            let rhs_a = m.get_awi(rhs);
            let rhs_b = m.get_dag(rhs);
            let out = m.next(1);
            match rng.next_u32() % 6 {
                0 => {
                    m.get_mut_awi(out).bool_(lhs_a.const_eq(&rhs_a).unwrap());
                    m.get_mut_dag(out).bool_(lhs_b.const_eq(&rhs_b).unwrap());
                }
                1 => {
                    m.get_mut_awi(out).bool_(lhs_a.const_ne(&rhs_a).unwrap());
                    m.get_mut_dag(out).bool_(lhs_b.const_ne(&rhs_b).unwrap());
                }
                2 => {
                    m.get_mut_awi(out).bool_(lhs_a.ult(&rhs_a).unwrap());
                    m.get_mut_dag(out).bool_(lhs_b.ult(&rhs_b).unwrap());
                }
                3 => {
                    m.get_mut_awi(out).bool_(lhs_a.ule(&rhs_a).unwrap());
                    m.get_mut_dag(out).bool_(lhs_b.ule(&rhs_b).unwrap());
                }
                4 => {
                    m.get_mut_awi(out).bool_(lhs_a.ilt(&rhs_a).unwrap());
                    m.get_mut_dag(out).bool_(lhs_b.ilt(&rhs_b).unwrap());
                }
                5 => {
                    m.get_mut_awi(out).bool_(lhs_a.ile(&rhs_a).unwrap());
                    m.get_mut_dag(out).bool_(lhs_b.ile(&rhs_b).unwrap());
                }
                _ => unreachable!(),
            }
        }
        // IsZero, IsUmax, IsImax, IsImin, IsUone
        22 => {
            let x = m.next4().1;
            let x_a = m.get_awi(x);
            let x_b = m.get_dag(x);
            let out = m.next(1);
            match rng.next_u32() % 5 {
                0 => {
                    m.get_mut_awi(out).bool_(x_a.is_zero());
                    m.get_mut_dag(out).bool_(x_b.is_zero());
                }
                1 => {
                    m.get_mut_awi(out).bool_(x_a.is_umax());
                    m.get_mut_dag(out).bool_(x_b.is_umax());
                }
                2 => {
                    m.get_mut_awi(out).bool_(x_a.is_imax());
                    m.get_mut_dag(out).bool_(x_b.is_imax());
                }
                3 => {
                    m.get_mut_awi(out).bool_(x_a.is_imin());
                    m.get_mut_dag(out).bool_(x_b.is_imin());
                }
                4 => {
                    m.get_mut_awi(out).bool_(x_a.is_uone());
                    m.get_mut_dag(out).bool_(x_b.is_uone());
                }
                _ => unreachable!(),
            }
        }
        // CountOnes, Lz, Tz, Sig
        23 => {
            let x = m.next4().1;
            let x_a = m.get_awi(x);
            let x_b = m.get_dag(x);
            let out = m.next_usize(usize::MAX);
            match rng.next_u32() % 4 {
                0 => {
                    m.get_mut_awi(out).usize_(x_a.count_ones());
                    m.get_mut_dag(out).usize_(x_b.count_ones());
                }
                1 => {
                    m.get_mut_awi(out).usize_(x_a.lz());
                    m.get_mut_dag(out).usize_(x_b.lz());
                }
                2 => {
                    m.get_mut_awi(out).usize_(x_a.tz());
                    m.get_mut_dag(out).usize_(x_b.tz());
                }
                3 => {
                    m.get_mut_awi(out).usize_(x_a.sig());
                    m.get_mut_dag(out).usize_(x_b.sig());
                }
                _ => unreachable!(),
            }
        }
        // LutSet
        24 => {
            let (entry_w, entry) = m.next4();
            let (inx_w, inx) = m.next4();
            let table_w = entry_w * (1 << inx_w);
            let table = m.next(table_w);
            let entry_a = m.get_awi(entry);
            let inx_a = m.get_awi(inx);
            m.get_mut_awi(table).lut_set(&entry_a, &inx_a).unwrap();
            let entry_b = m.get_dag(entry);
            let inx_b = m.get_dag(inx);
            m.get_mut_dag(table).lut_set(&entry_b, &inx_b).unwrap();
        }
        // Resize
        25 => {
            let lhs = m.next4().1;
            let rhs = m.next4().1;
            let b = m.next(1);
            let rhs_a = m.get_awi(rhs);
            let b_a = m.get_awi(b);
            m.get_mut_awi(lhs).resize_(&rhs_a, b_a.to_bool());
            let rhs_b = m.get_dag(rhs);
            let b_b = m.get_dag(b);
            m.get_mut_dag(lhs).resize_(&rhs_b, b_b.to_bool());
        }
        // ZeroResizeOverflow, SignResizeOverflow
        26 => {
            let lhs = m.next4().1;
            let rhs = m.next4().1;
            let out = m.next(1);
            let mut lhs_a = m.get_awi(lhs);
            let rhs_a = m.get_awi(rhs);
            let mut lhs_b = m.get_dag(lhs);
            let rhs_b = m.get_dag(rhs);
            match rng.next_u32() % 2 {
                0 => {
                    m.get_mut_awi(out).bool_(lhs_a.zero_resize_(&rhs_a));
                    m.get_mut_dag(out).bool_(lhs_b.zero_resize_(&rhs_b));
                }
                1 => {
                    m.get_mut_awi(out).bool_(lhs_a.sign_resize_(&rhs_a));
                    m.get_mut_dag(out).bool_(lhs_b.sign_resize_(&rhs_b));
                }
                _ => unreachable!(),
            }
        }
        // ArbMulAdd
        27 => {
            let (w, lhs) = m.next4();
            match rng.next_u32() % 3 {
                0 => {
                    let rhs = m.next(w);
                    let out = m.next(w);
                    let lhs_a = m.get_awi(lhs);
                    let rhs_a = m.get_awi(rhs);
                    let lhs_b = m.get_dag(lhs);
                    let rhs_b = m.get_dag(rhs);
                    m.get_mut_awi(out).mul_add_(&lhs_a, &rhs_a).unwrap();
                    m.get_mut_dag(out).mul_add_(&lhs_b, &rhs_b).unwrap();
                }
                1 => {
                    let rhs = m.next4().1;
                    let out = m.next4().1;
                    let lhs_a = m.get_awi(lhs);
                    let rhs_a = m.get_awi(rhs);
                    let lhs_b = m.get_dag(lhs);
                    let rhs_b = m.get_dag(rhs);
                    m.get_mut_awi(out).arb_umul_add_(&lhs_a, &rhs_a);
                    m.get_mut_dag(out).arb_umul_add_(&lhs_b, &rhs_b);
                }
                2 => {
                    let rhs = m.next4().1;
                    let out = m.next4().1;
                    let mut lhs_a = m.get_awi(lhs);
                    let mut rhs_a = m.get_awi(rhs);
                    let mut lhs_b = m.get_dag(lhs);
                    let mut rhs_b = m.get_dag(rhs);
                    m.get_mut_awi(out).arb_imul_add_(&mut lhs_a, &mut rhs_a);
                    m.get_mut_dag(out).arb_imul_add_(&mut lhs_b, &mut rhs_b);
                }
                _ => unreachable!(),
            }
        }
        // Mux
        28 => {
            let (w, lhs) = m.next4();
            let rhs = m.next(w);
            let b = m.next(1);
            let rhs_a = m.get_awi(rhs);
            let b_a = m.get_awi(b);
            m.get_mut_awi(lhs).mux_(&rhs_a, b_a.to_bool()).unwrap();
            let rhs_b = m.get_dag(rhs);
            let b_b = m.get_dag(b);
            m.get_mut_dag(lhs).mux_(&rhs_b, b_b.to_bool()).unwrap();
        }
        // UQuo, URem, IQuo, IRem
        29 => {
            let (w, duo) = m.next4();
            let div = m.next(w);
            let quo = m.next(w);
            let rem = m.next(w);
            let out0 = m.next(w);
            let out1 = m.next(w);

            if m.get_awi(div).is_zero() {
                m.get_mut_awi(div).uone_();
                m.get_mut_dag(div).uone_();
            }

            let mut duo_a = m.get_awi(duo);
            let mut div_a = m.get_awi(div);
            let mut quo_a = m.get_awi(quo);
            let mut rem_a = m.get_awi(rem);
            let mut duo_b = m.get_dag(duo);
            let mut div_b = m.get_dag(div);
            let mut quo_b = m.get_dag(quo);
            let mut rem_b = m.get_dag(rem);
            match rng.next_u32() % 2 {
                0 => {
                    awi::Bits::udivide(&mut quo_a, &mut rem_a, &duo_a, &div_a).unwrap();
                    dag::Bits::udivide(&mut quo_b, &mut rem_b, &duo_b, &div_b).unwrap();
                }
                1 => {
                    awi::Bits::idivide(&mut quo_a, &mut rem_a, &mut duo_a, &mut div_a).unwrap();
                    dag::Bits::idivide(&mut quo_b, &mut rem_b, &mut duo_b, &mut div_b).unwrap();
                }
                _ => unreachable!(),
            }
            m.get_mut_awi(out0).copy_(&quo_a).unwrap();
            m.get_mut_awi(out1).copy_(&rem_a).unwrap();
            m.get_mut_dag(out0).copy_(&quo_b).unwrap();
            m.get_mut_dag(out1).copy_(&rem_b).unwrap();
        }
        // Repeat
        30 => {
            let lhs = m.next4().1;
            let rhs = m.next4().1;
            let rhs_a = m.get_awi(rhs);
            m.get_mut_awi(lhs).repeat_(&rhs_a);
            let rhs_b = m.get_dag(rhs);
            m.get_mut_dag(lhs).repeat_(&rhs_b);
        }
        _ => unreachable!(),
    }
}

#[test]
fn dag_fuzzing() {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    let mut m = Mem::new();

    for _ in 0..N.1 {
        let epoch = Epoch::new();
        m.clear();
        for _ in 0..N.0 {
            num_dag_duo(&mut rng, &mut m)
        }
        m.finish(&epoch);
        m.eval_and_verify_equal(&epoch);
        drop(epoch);
    }
}
