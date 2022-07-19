use std::num::NonZeroUsize;

use awint::prelude as num;
use awint_dag::{
    common::{EvalError, Lineage, Op},
    lowering::Dag,
    prelude as dag,
};
use awint_internals::BITS;
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};
use triple_arena::{ptr_trait_struct_with_gen, Arena, Ptr};

// miri is just here to check that the unsized deref hacks are working
const N: u32 = if cfg!(miri) {
    32
} else if cfg!(debug_assertions) {
    // TODO increase
    1000
} else {
    100000
};

ptr_trait_struct_with_gen!(P0);

#[derive(Debug, Clone)]
struct Pair {
    num: num::ExtAwi,
    dag: dag::ExtAwi,
}

impl Pair {
    pub fn new(lit: num::ExtAwi) -> Self {
        Self {
            num: lit.clone(),
            dag: lit.const_as_ref().into(),
        }
    }
}

#[derive(Debug)]
struct Mem {
    a: Arena<P0, Pair>,
    // The outer Vec has 65 Vecs for all the supported bitwidths (there is a dummy 0 bitwidth Vec
    // and one for each of 1..=64), the inner Vecs are unsorted and used for random querying
    v: Vec<Vec<Ptr<P0>>>,
    rng: Xoshiro128StarStar,
}

impl Mem {
    pub fn new() -> Self {
        let mut v = vec![];
        for _ in 0..65 {
            v.push(vec![]);
        }
        Self {
            a: Arena::<P0, Pair>::new(),
            v,
            rng: Xoshiro128StarStar::seed_from_u64(0),
        }
    }

    /*pub fn new_val(&mut self, w: usize, val: usize) -> Ptr<P0> {
        if (w == 0) || (w > 64) {
            panic!("width needs to be in range 1..=64");
        }
        let p = self.a.insert(Pair::new(num::ExtAwi::from_usize(val)));
        self.v[w].push(p);
        p
    }

    /// Randomly generates a width `w` integer upper bounded by `cap`
    pub fn new_capped(&mut self, w: usize, cap: usize) -> Ptr<P0> {
        let tmp = (self.rng.next_u64() as usize) % cap;
        self.new_val(w, tmp)
    }*/

    /// Randomly creates a new pair or gets an existing one under the `cap`
    pub fn next_capped(&mut self, w: usize, cap: usize) -> Ptr<P0> {
        let try_query = (self.rng.next_u32() % 4) != 0;
        if try_query && (!self.v[w].is_empty()) {
            let p = self.v[w][(self.rng.next_u32() as usize) % self.v[w].len()];
            if self.get_num(p).to_usize() < cap {
                return p
            }
        }
        let mut lit = num::ExtAwi::zero(NonZeroUsize::new(w).unwrap());
        lit.rand_assign_using(&mut self.rng).unwrap();
        let tmp = lit.to_usize() % cap;
        lit.usize_assign(tmp);
        let p = self.a.insert(Pair::new(lit));
        self.v[w].push(p);
        p
    }

    /// Randomly creates a new pair or gets an existing one
    pub fn next(&mut self, w: usize) -> Ptr<P0> {
        let try_query = (self.rng.next_u32() % 4) != 0;
        if try_query && (!self.v[w].is_empty()) {
            self.v[w][(self.rng.next_u32() as usize) % self.v[w].len()]
        } else {
            let mut lit = num::ExtAwi::zero(NonZeroUsize::new(w).unwrap());
            lit.rand_assign_using(&mut self.rng).unwrap();
            let p = self.a.insert(Pair::new(lit));
            self.v[w].push(p);
            p
        }
    }

    /// Calls `next` with a random integer in 1..5, returning a tuple of the
    /// width chosen and the Ptr to what `next` returned.
    pub fn next1_5(&mut self) -> (usize, Ptr<P0>) {
        let w = ((self.rng.next_u32() as usize) % 4) + 1;
        (w, self.next(w))
    }

    pub fn next_usize(&mut self, cap: usize) -> Ptr<P0> {
        self.next_capped(BITS, cap)
    }

    // just use cloning for the immutable indexing, because dealing with the guards
    // of mixed internal mutability is too much. We can't get the signature of
    // `Index` to work in any case.

    pub fn get_num(&self, inx: Ptr<P0>) -> num::ExtAwi {
        self.a[inx].num.clone()
    }

    pub fn get_dag(&self, inx: Ptr<P0>) -> dag::ExtAwi {
        self.a[inx].dag.unstable_clone_identical()
    }

    pub fn get_mut_num(&mut self, inx: Ptr<P0>) -> &mut num::ExtAwi {
        &mut self.a[inx].num
    }

    pub fn get_mut_dag(&mut self, inx: Ptr<P0>) -> &mut dag::ExtAwi {
        &mut self.a[inx].dag
    }

    /// Makes sure that plain evaluation works
    pub fn eval_and_verify_equal(&mut self) -> Result<(), EvalError> {
        for pair in self.a.vals() {
            let (mut dag, res) = Dag::<P0>::new(&[pair.dag.state()], &[pair.dag.state()]);
            res?;
            let leaf = dag.noted[0];
            dag.eval_tree(leaf)?;
            if let Op::Literal(ref lit) = dag[leaf].op {
                if pair.num != *lit {
                    return Err(EvalError::OtherStr("real and mimick mismatch"))
                }
            }
        }
        Ok(())
    }

    /// Makes sure that lowering works
    pub fn lower_and_verify_equal(&mut self) -> Result<(), EvalError> {
        for pair in self.a.vals() {
            let (mut dag, res) = Dag::<P0>::new(&[pair.dag.state()], &[pair.dag.state()]);
            res?;
            let leaf = dag.noted[0];
            dag.lower()?;
            dag.eval_tree(leaf)?;
            if let Op::Literal(ref lit) = dag[leaf].op {
                if pair.num != *lit {
                    return Err(EvalError::OtherStr("real and mimick mismatch"))
                }
            }
        }
        Ok(())
    }
}

#[test]
fn dag_fuzzing() {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    let mut m = Mem::new();

    for _ in 0..N {
        match rng.next_u32() % 9 {
            0 => {
                let (out_w, out) = m.next1_5();
                let (inx_w, inx) = m.next1_5();
                let lut = m.next(out_w * (1 << inx_w));
                let lut_a = m.get_num(lut);
                let inx_a = m.get_num(inx);
                m.get_mut_num(out).lut(&lut_a, &inx_a).unwrap();
                let lut_b = m.get_dag(lut);
                let inx_b = m.get_dag(inx);
                m.get_mut_dag(out).lut(&lut_b, &inx_b).unwrap();
            }
            1 => {
                let (bits_w, bits) = m.next1_5();
                let inx = m.next_usize(bits_w);
                let out = m.next(1);
                let bits_a = m.get_num(bits);
                let inx_a = m.get_num(inx);
                m.get_mut_num(out)
                    .bool_assign(bits_a.get(inx_a.to_usize()).unwrap());
                let bits_b = m.get_dag(bits);
                let inx_b = m.get_dag(inx);
                m.get_mut_dag(out)
                    .bool_assign(bits_b.get(inx_b.to_usize()).unwrap());
            }
            2 => {
                let (bits_w, bits) = m.next1_5();
                let inx = m.next_usize(bits_w);
                let bit = m.next(1);
                let inx_a = m.get_num(inx);
                let bit_a = m.get_num(bit);
                m.get_mut_num(bits)
                    .set(inx_a.to_usize(), bit_a.to_bool())
                    .unwrap();
                let inx_b = m.get_dag(inx);
                let bit_b = m.get_dag(bit);
                m.get_mut_dag(bits)
                    .set(inx_b.to_usize(), bit_b.to_bool())
                    .unwrap();
            }
            3 => {
                let (lhs_w, lhs) = m.next1_5();
                let to = m.next_usize(lhs_w);
                let (rhs_w, rhs) = m.next1_5();
                let from = m.next_usize(rhs_w);
                let to_a = m.get_num(to);
                let rhs_a = m.get_num(rhs);
                let from_a = m.get_num(from);
                m.get_mut_num(lhs)
                    .field_bit(to_a.to_usize(), &rhs_a, from_a.to_usize())
                    .unwrap();
                let to_b = m.get_dag(to);
                let rhs_b = m.get_dag(rhs);
                let from_b = m.get_dag(from);
                m.get_mut_dag(lhs)
                    .field_bit(to_b.to_usize(), &rhs_b, from_b.to_usize())
                    .unwrap();
            }
            4 => {
                let lhs = m.next1_5().1;
                let rhs = m.next1_5().1;
                let rhs_a = m.get_num(rhs);
                m.get_mut_num(lhs).zero_resize_assign(&rhs_a);
                let rhs_b = m.get_dag(rhs);
                m.get_mut_dag(lhs).zero_resize_assign(&rhs_b);
            }
            5 => {
                let lhs = m.next1_5().1;
                let rhs = m.next1_5().1;
                let rhs_a = m.get_num(rhs);
                m.get_mut_num(lhs).sign_resize_assign(&rhs_a);
                let rhs_b = m.get_dag(rhs);
                m.get_mut_dag(lhs).sign_resize_assign(&rhs_b);
            }
            6 => {
                let x = m.next1_5().1;
                m.get_mut_num(x).not_assign();
                m.get_mut_dag(x).not_assign();
            }
            7 => {
                let (lhs_w, lhs) = m.next1_5();
                let rhs = m.next(lhs_w);
                let rhs_a = m.get_num(rhs);
                let rhs_b = m.get_dag(rhs);
                match rng.next_u32() % 3 {
                    0 => {
                        m.get_mut_num(lhs).or_assign(&rhs_a);
                        m.get_mut_dag(lhs).or_assign(&rhs_b);
                    }
                    1 => {
                        m.get_mut_num(lhs).and_assign(&rhs_a);
                        m.get_mut_dag(lhs).and_assign(&rhs_b);
                    }
                    2 => {
                        m.get_mut_num(lhs).xor_assign(&rhs_a);
                        m.get_mut_dag(lhs).xor_assign(&rhs_b);
                    }
                    _ => unreachable!(),
                }
            }
            8 => {
                let cin = m.next(1);
                let (lhs_w, lhs) = m.next1_5();
                let rhs = m.next(lhs_w);
                let out = m.next(lhs_w);
                let unsigned = m.next(1);
                let signed = m.next(1);

                let cin_a = m.get_num(cin);
                let lhs_a = m.get_num(lhs);
                let rhs_a = m.get_num(rhs);
                let overflow = m
                    .get_mut_num(out)
                    .cin_sum_assign(cin_a.to_bool(), &lhs_a, &rhs_a)
                    .unwrap();
                m.get_mut_num(unsigned).bool_assign(overflow.0);
                m.get_mut_num(signed).bool_assign(overflow.1);

                let cin_b = m.get_dag(cin);
                let lhs_b = m.get_dag(lhs);
                let rhs_b = m.get_dag(rhs);
                let overflow = m
                    .get_mut_dag(out)
                    .cin_sum_assign(cin_b.to_bool(), &lhs_b, &rhs_b)
                    .unwrap();
                m.get_mut_dag(unsigned).bool_assign(overflow.0);
                m.get_mut_dag(signed).bool_assign(overflow.1);
            }
            _ => unreachable!(),
        }
    }
    m.eval_and_verify_equal().unwrap();
    m.lower_and_verify_equal().unwrap();
    /*let dag = &m.a.vals().next().unwrap().dag;
    let (mut dag, res) = Dag::<P0>::new(&[dag.state()], &[dag.state()]);
    dag.render_to_svg_file(std::path::PathBuf::from("rendered.svg")).unwrap();
    res.unwrap();*/
    //dbg!(m);
    //panic!();
}
