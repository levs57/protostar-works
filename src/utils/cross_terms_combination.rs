// Given values of gates gate_i(x+ty) in 0,1,...,d_i, compute their combination.
// Coefficients of the combination are all monomials for a logarithmtic amount of challenge variables.
// This is a bit similar to protogalaxy; main reason why we are doing it is to skip a commitment
// to an additional round.

use std::{iter::{once, zip}, fmt::Debug};

use super::field_precomp::FieldUtils;
use itertools::Itertools;

/// A utility function that guarantees that chunks will always have the length divisble by align_by.
/// Useful if you'd like to execute some operations that involve more than a single element (and do not want to transmute).
pub fn parallelize_with_alignment<T: Send, F: Fn(&mut [T], &mut [T], usize) + Send + Sync + Clone>(v: &mut [T], w: &mut [T], f: F, align_v: usize, align_w: usize) {
    
    assert!(v.len()%align_v == 0);
    assert!(w.len()%align_w == 0);
    let f = &f;
    let total_iters = v.len()/align_v;
    assert!(total_iters == w.len()/align_w);
    let num_threads = rayon_core::current_num_threads();
    let base_chunk_size = total_iters / num_threads;
    let cutoff_chunk_id = total_iters % num_threads;
    let split_pos = cutoff_chunk_id * (base_chunk_size + 1);
    let (v_hi, v_lo) = v.split_at_mut(split_pos*align_v);
    let (w_hi, w_lo) = w.split_at_mut(split_pos*align_w);

    rayon_core::scope(|scope| {
        // Skip special-case: number of iterations is cleanly divided by number of threads.
        if cutoff_chunk_id != 0 {
            for (chunk_id, (chunk_v, chunk_w)) in v_hi
                .chunks_exact_mut((base_chunk_size + 1)*align_v).zip_eq(
                    w_hi.chunks_exact_mut((base_chunk_size + 1)*align_w)
                )
                .enumerate() {
                let offset = chunk_id * (base_chunk_size + 1);
                scope.spawn(move |_| f(chunk_v, chunk_w, offset));
            }
        }
        // Skip special-case: less iterations than number of threads.
        if base_chunk_size != 0 {
            for (chunk_id, (chunk_v, chunk_w)) in v_lo
                .chunks_exact_mut(base_chunk_size*align_v).zip_eq(
                    w_lo.chunks_exact_mut(base_chunk_size*align_w)
                )
                .enumerate() {
                let offset = split_pos + chunk_id * base_chunk_size;
                scope.spawn(move |_| f(chunk_v, chunk_w, offset));
            }
        }
    });
}

fn compute_binomial_coefficients(up_to: usize) -> Vec<Vec<u64>> {
    assert!(up_to < 66, "Binomial coefficients of such size do not fit in u64.");
    let mut ret : Vec<_> = (0..up_to).map(|i| Vec::with_capacity(i+1)).collect();
    ret[0].push(1);
    ret[1].push(1); ret[1].push(1);

    for i in 2..up_to {
        ret[i].push(1);
        for j in 0..i-1 {
            let tmp = ret[i-1][j] + ret[i-1][j+1];
            ret[i].push(tmp);
        }
        ret[i].push(1);
    }

    ret
}

/// Computes value of a degree d polynomial in point d+1, given values in 0..d.
/// Assumes that binom is a list of binomial coefficients of length 1 larger than vals
/// (d-th index in the pascal triangle)
fn extend<F:FieldUtils> (vals: &[F], binom: &[u64]) -> F {
    assert!(vals.len() + 1 == binom.len());

    let ret = vals.iter().zip(binom.iter())
        .map(|(v, c)| v.scale(*c))
        .enumerate()
        .fold(F::ZERO, |acc, (i, upd)| {
            if i%2==0 {
                acc + upd
            } else {
                acc - upd
            }
        });

    if vals.len() % 2 == 0 { -ret } else { ret }
}

#[derive(Clone)]
pub struct EvalLayout {
    pub deg : usize,
    pub amount : usize,
}

impl Debug for EvalLayout{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(d: {}) * {}", self.deg, self.amount)
    }
}


pub trait SanitizeLayout {
    fn total_size(&self) -> usize;
    fn check(&self) -> bool;
    fn num_polys(&self) -> usize;
}

impl SanitizeLayout for Vec<EvalLayout> {
    fn total_size(&self) -> usize {
        self.iter().fold(0,|acc, upd| acc+(upd.deg+1)*upd.amount)
    }

    fn check(&self) -> bool {
        let l = self.len();
        let mut flag = true;
        if l>1 {
            for i in 0..l-1 {
                flag &= self[i].deg < self[i+1].deg;
            }
        }        
        flag
    }

    fn num_polys(&self) -> usize {
        self.iter().fold(0, |acc, upd| acc+upd.amount)
    }
}

/// Computes the layout of all phases of merging from the first phase.
fn compute_layouts(layout: Vec<EvalLayout>, num_vars: usize)->Vec<Vec<EvalLayout>>{
    let mut layouts = vec![];
    let l = layout.len();
    layouts.push(layout);
    for i in 0..num_vars {
        let mut tmp = vec![];
        let mut carry = 0;

        for EvalLayout{ deg, amount } in &layouts[i] {
            let amount = amount+carry;
            carry = amount%2;
            tmp.push(EvalLayout{ deg: deg+1, amount : amount/2 });
        }
        tmp[l-1].amount += carry;
        layouts.push(tmp);
    }
    layouts
}

/// Merges a and b and writes the result in target.
fn merge<F: FieldUtils>(a: &[F], b: &[F], target: &mut [F], zip_with: &[F], binom: &[u64]) -> () {
    debug_assert!(a.len() == b.len());
    debug_assert!(zip_with.len() == a.len()+1);
    debug_assert!(target.len() == a.len()+1);    

    let ae = extend(a, binom);
    let be = extend(b, binom);

    let ae = a.iter().chain(once(&ae));
    let be = b.iter().chain(once(&be)).zip_eq(zip_with).map(|(x,y)|*x*y);
    
    let t = ae.zip_eq(be).map(|(x,y)|*x+y);
    target.iter_mut().zip_eq(t)
        .map(|(x, y)|{
            *x = y;            
        }).count();

}

/// Evals is a list of all the coefficients, in increasing degree order.
/// Layout is pairs (degree, amount) - what is the amount of polynomials of degree d.
/// Point is a sequence of challenges, given in evaluation form - i.e. these are actually values of a_i(t) in 0 and 1.
pub fn combine_cross_terms<F: FieldUtils>(evals: Vec<F>, layout: Vec<EvalLayout>, point: Vec<[F;2]>) -> Vec<F> {
    let l = layout.len();
    assert!(layout.check(), "Degrees must strictly increase.");
    assert!(layout[l-1].amount>0, "Layout must not have trailing zero elements.");
    let total_size = layout.total_size();
    assert!(evals.len() == total_size, "Total length must be sum of all lengths of all polynomials");
    let num_vars = point.len();
    let num_polys = layout.num_polys();
    if num_vars == 0 {
        assert!(l == 1);
        assert!(num_polys == 1);
        //return evals;
    }
    assert!(num_polys <= 1<<num_vars, "Not enough dimensions.");
    assert!(num_polys > 1<<(num_vars-1), "Too many dimensions.");
    let binoms = compute_binomial_coefficients(30);
    let layouts = compute_layouts(layout, num_vars);

    struct Evals<F> {
        data: [Vec<F>; 2],
        current: usize,
    }
    let mut data = Evals {
        data: [evals, vec![F::ZERO; layouts[1].total_size()]],
        current: 0,
    };

    impl<F> Evals<F> {
        fn get_st_pair(&mut self, i: usize) -> (&mut [F], &mut [F]) {
            let ([a], [b]) = self.data.split_at_mut(1) else { unreachable!() };
            let res;
            if self.current == 0 {
                res = (a.as_mut_slice(), b.as_mut_slice());
            } else {
                res = (b.as_mut_slice(), a.as_mut_slice());
            }
            self.current = (self.current + 1) % 2;
            return res;
        }
    }

    for i in 0..num_vars {
        let source_layout = &layouts[i];
        let target_layout = &layouts[i+1];
        let (mut source_evals_full, mut target_evals_full) = data.get_st_pair(i);

        let mut carry_poly : Vec<F> = vec![];
        let mut carry_flag = false;
        let mut pt_vals = point[i].to_vec();
        let ptinc = point[i][1]-point[i][0];

        for (q, (EvalLayout{deg: sd, amount : sa}, EvalLayout{deg: td, amount : ta})) in source_layout.iter().zip_eq(target_layout.iter()).enumerate() {

            if *sa == 0 {
                continue
            }
            
            let mut source_evals;
            (source_evals, source_evals_full) = source_evals_full.split_at_mut((sd+1)*sa);
            let mut target_evals;
            (target_evals, target_evals_full) = target_evals_full.split_at_mut((td+1)*ta);

            while pt_vals.len() < sd+2 {
                let last = pt_vals[pt_vals.len()-1];
                pt_vals.push(last+ptinc);
            } 

            // Process carry by taking a single chunk from source_evals. It is guaranteed that it is nonempty.
            if carry_flag {
                let mut counter = carry_poly.len();
                while counter < sd+1 {
                    let inc = extend(&carry_poly, &binoms[carry_poly.len()]);
                    carry_poly.push(inc);
                    counter+=1;
                }

                let source_chunk;
                (source_chunk, source_evals) = source_evals.split_at_mut(sd+1);
                let target_chunk;
                (target_chunk, target_evals) = target_evals.split_at_mut(td+1);
                merge(&carry_poly, source_chunk, target_chunk, &pt_vals, &binoms[sd+1]);
            }

            // Create a new carry if needed.
            let l = source_evals.len();
            if (l/(sd+1))%2 == 1 {
                carry_flag = true;
                let tmp;
                (source_evals, tmp) = source_evals.split_at_mut(l-(sd+1));
                carry_poly = tmp.to_vec();
                // On the last step, just write the carry where it belongs.
                if q == target_layout.len()-1 {
                    let l = target_evals.len();
                    let tmp;
                    (target_evals, tmp) = target_evals.split_at_mut(l-(td+1));
                    let ext = extend(&carry_poly, &binoms[carry_poly.len()]);
                    tmp.iter_mut().zip_eq(carry_poly.iter().chain(once(&ext))).map(|(x,y)| *x=*y).count();
                }

            } else {
                carry_flag = false;
            }

            // The main parallelized copying - merge source evals.
            parallelize_with_alignment(
                source_evals, 
                target_evals,
                |source_evals, target_evals, _| {
                    let mut l = 1;
                    let mut source_evals = source_evals;
                    let mut target_evals = target_evals;
                    while l > 0 {
                        let source;
                        (source, source_evals) = source_evals.split_at_mut(2*(sd+1));
                        let target;
                        (target, target_evals) = target_evals.split_at_mut(td+1);
                        l = source_evals.len();
                        let (a,b) = source.split_at_mut(sd+1);
                        merge(a, b, target, &pt_vals, &binoms[sd+1]);
                    }
                }, 
                2*(sd+1), 
                td+1
            );
        }
    }

    data.data[data.current].clone()
    // evals[num_vars].clone()
}

#[cfg(test)]
mod tests {
    use std::iter::repeat_with;
    use ff::Field;
    use halo2::halo2curves::bn256;
    use itertools::Itertools;
    use rand_core::OsRng;


    use crate::{utils::{cross_terms_combination::{parallelize_with_alignment, compute_binomial_coefficients, extend, compute_layouts, SanitizeLayout}, field_precomp::FieldUtils}, gadgets::range::lagrange_choice_batched};

    use super::{combine_cross_terms, EvalLayout, merge};

    type Fr = bn256::Fr;
    
    fn ev_poly<F: FieldUtils>(p: &[F], x: F) -> F {
        lagrange_choice_batched(x, p.len() as u64)
            .into_iter()
            .zip_eq(p.iter())
            .fold(F::ZERO,|acc, (a, b)|acc+a*b)
    }

    #[test]
    fn test_parallelize_with_alignment() -> () {
        println!("Current threads: {}", rayon_core::current_num_threads());

        let mut arr1 : Vec<_> = (0..13*7).collect();
        let mut arr2 : Vec<_> = (0..13*5).collect();

        parallelize_with_alignment(&mut arr1, &mut arr2, |chunk_v, chunk_w, offset|{
            chunk_v.iter_mut().enumerate().map(|(i, x)| *x -= (i + offset * 7)).count();
            chunk_w.iter_mut().enumerate().map(|(i, x)| *x -= (i + offset * 5)).count();
        }, 7, 5);

        for v in arr1 {assert!(v==0)}
        for v in arr2 {assert!(v==0)}
    }

    #[test]

    fn test_extension() -> (){
        fn test_poly(x:Fr) -> Fr {
            Fr::from(5) 
            + Fr::from(6735)*x 
            + Fr::from(420)*x*x 
            + Fr::from(32687)*x*x*x 
            + Fr::from(1212)*x*x*x*x
        }

        let vals : Vec<Fr> = (0..5).map(|i|test_poly(Fr::from(i))).collect();

        let binoms = compute_binomial_coefficients(10);

        println!("{:?}", binoms);

        assert!(extend(&vals, &binoms[5]) == test_poly(Fr::from(5)));
    }

    #[test]
    
    fn test_layout()->() {
        let layout = [(33, 2), (17, 3), (7, 7)].into_iter().map(|(amount, deg)| EvalLayout{deg, amount}).collect();
        println!("{:?}", compute_layouts(layout, 6));
    }


    #[test]

    fn test_merge()->() {
        let k = 9;

        let p1 : Vec<_> = repeat_with(|| Fr::random(OsRng)).take(k).collect();
        let p2 : Vec<_> = repeat_with(|| Fr::random(OsRng)).take(k).collect();
        let mut pt : Vec<_> = repeat_with(|| Fr::random(OsRng)).take(2).collect();

        let r = Fr::random(OsRng);

        let lhs = ev_poly(&p1, r) + ev_poly(&p2, r) * ev_poly(&pt, r);

        let inc = pt[1]-pt[0];

        for _ in 0..k-1 {
            let last = pt[pt.len()-1];
            pt.push(last+inc);
        }

        let binomials = compute_binomial_coefficients(15);

        let mut v = vec![Fr::ZERO; k+1];
        merge(&p1,&p2,&mut v, &pt, &binomials[k]);

        let rhs = ev_poly(&v, r);

        assert!(lhs == rhs);
        
    }

    #[test]

    fn test_cross_terms_combination()->() {
        let params = [(1000000, 0, 1),];
        for (x,y,z) in params {
        if x+y+z<2 {continue}
        let layout : Vec<_> = [(x, 2), (y, 3), (z, 7)].into_iter().map(|(amount, deg)| EvalLayout{deg, amount}).collect();

        let p2 : Vec<_> = repeat_with(
            ||repeat_with(
                ||Fr::random(OsRng))
                .take(3)
                .collect::<Vec<Fr>>()
            ).take(x)
            .collect();

            let p3 : Vec<_> = repeat_with(
            ||repeat_with(
                ||Fr::random(OsRng))
                .take(4)
                .collect::<Vec<Fr>>()
            ).take(y)
            .collect();

            let p7 : Vec<_> = repeat_with(
                ||repeat_with(
                    ||Fr::random(OsRng))
                    .take(8)
                    .collect::<Vec<Fr>>()
                ).take(z)
                .collect();
    
        let mut num_vars = 0;
        let mut tmp = layout.num_polys()-1;

        while tmp>0 { num_vars+=1; tmp>>=1}

        let pts : Vec<_> = repeat_with(||[Fr::random(OsRng), Fr::random(OsRng)]).take(num_vars).collect();
        
        let challenge = Fr::random(OsRng);

        let pts_evals : Vec<_> = pts.iter().map(|l|ev_poly(l, challenge)).collect();
        let hypercube_evals = (0..1<<num_vars).map(|i| { 
            let mut b = i;
            let mut ret = Fr::ONE;
            for j in 0..num_vars {
                if b%2 == 1 {ret *= pts_evals[j]};
                b>>=1;
            }
            ret
        });

        let p2_evals = p2.iter().map(|p| ev_poly(p, challenge));
        let p3_evals = p3.iter().map(|p| ev_poly(p, challenge));
        let p7_evals = p7.iter().map(|p| ev_poly(p, challenge));
        let p_evals : Vec<_> = p2_evals.chain(p3_evals).chain(p7_evals).collect();

        let lhs = p_evals.iter().zip(hypercube_evals).fold(Fr::ZERO, |acc, (x,y)| acc+x*y);

        let evals : Vec<_> = p2.into_iter().chain(p3).chain(p7).flatten().collect();
        let combined = combine_cross_terms(evals, layout, pts);
        let rhs = ev_poly(&combined, challenge);

        assert_eq!(lhs, rhs);
    }
    }
}

