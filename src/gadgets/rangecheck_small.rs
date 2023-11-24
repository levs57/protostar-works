use std::{rc::Rc, iter::repeat};

use ff::PrimeField;
use crate::{utils::field_precomp::FieldUtils, gate::Gatebb, circuit::{Circuit, Advice, PolyOp}, constraint_system::Variable};
use num_bigint::{self, BigUint};

use super::rangecheck_common::{limb_decompose_unchecked, VarRange};

pub fn rangecheck<F: PrimeField+FieldUtils>(x: F, range: u64) -> F {
    let x = x - F::TWO_INV.scale(range-1);
    let y = x.square();
    let ret = if range%2 == 0 {
        (0..range/2).fold(F::ONE, |acc, i| acc*(y-F::half_square(1+2*i)))    
    } else {
        (0..range/2).fold(x, |acc, i| acc*(y - F::half_square(2+2*i)))
    };

    ret
}

/// A polynomial of x which has value 0 in all values 0..range except t, in which it = 1.
pub fn lagrange_choice<F: PrimeField+FieldUtils>(x: F, t: u64, range: u64) -> F{
    let q = x - F::TWO_INV.scale(range-1);
    let y = q.square();
    let ret = if range%2 == 0 {
        (0..range/2).fold(
            F::ONE, 
            |acc, i| {
                if t != i+range/2 && range-1-t != i+range/2 {
                    acc*(y-F::half_square(1+2*i))
                } else {
                    acc*(x - F::from(range-1-t))
                }
            }
        )
    } else {
        if 2*t == range-1 {
            (0..range/2)
                .fold(F::ONE, |acc, i| acc*(y - F::half_square(2+2*i)))
        } else {
            (0..range/2)
                .fold(q, |acc, i| {
                    if t != 1+i+(range-1)/2 && range-1-t != 1+i+(range-1)/2 {
                        acc*(y - F::half_square(2+2*i))
                    } else {
                        acc*(x - F::from(range-1-t))
                    }
                }
            )
        }
    };

    ret * F::inv_lagrange_prod(t, range)
}

/// Returns the vector of values of lagrange interpolation polynomials on the set 0..n.
pub fn lagrange_choice_batched<F: PrimeField+FieldUtils>(x: F, n: u64) -> Vec<F> {
    
    let ret = if n>3 { // General case
        
        let q = x - F::TWO_INV.scale(n-1);
        let y = q.square();
        let n = n as usize;

        let ret = if n%2 == 0 {
            let l = (n/2) as usize;
            let mut sqrs = vec![];
            for i in 0..l {
                sqrs.push(y - F::half_square((1+2*i) as u64));
            }
            let mut prod_l = vec![];
            let mut prod_r = vec![];
            
            for i in 0..l-1 {
                if i == 0 {
                    prod_l.push(sqrs[0]);
                    prod_r.push(sqrs[l-1]);
                } else {
                    prod_l.push(prod_l[i-1]*sqrs[i]);
                    prod_r.push(prod_r[i-1]*sqrs[l-1-i]);
                }
            }

            let mut ret : Vec<F> = vec![];

            for t in 0..n {
                let i = if t >= l {t - l} else {l - 1 - t};
                
                ret.push(
                    (x-F::from((n-t-1) as u64)) *
                    if i == 0 { prod_r[l-2] } else if i == l-1 {prod_l[l-2]} else {prod_l[i-1] * prod_r[l-2-i]}
                );
            }
            ret
        } else {
            let l = (n/2) as usize;
            let mut sqrs = vec![];
            for i in 0..l {
                sqrs.push(y - F::half_square((2+2*i) as u64));
            }
            let mut prod_l = vec![];
            let mut prod_r = vec![];
            
            for i in 0..l-1 {
                if i == 0 {
                    prod_l.push(sqrs[0]*q);
                    prod_r.push(sqrs[l-1]);
                } else {
                    prod_l.push(prod_l[i-1]*sqrs[i]);
                    prod_r.push(prod_r[i-1]*sqrs[l-1-i]);
                }
            }

            let prod = prod_r[l-2]*sqrs[0]; // full product without q
            let mut ret : Vec<F> = vec![];
            for t in 0..n {
                if t == l {
                    ret.push(prod);
                } else {
                    let i = if t > l {t - l - 1} else {l - 1 - t};
                    ret.push(
                        (x-F::from((n-t-1) as u64)) *
                        if i == 0 { q * prod_r[l-2] } else if i == l-1 {prod_l[l-2]} else {prod_l[i-1] * prod_r[l-2-i]}
                    );
                }
            }
            ret
        };
        ret
    } else if n == 3 {
        let x1 = x-F::from(1);
        let x2 = x-F::from(2);
        vec![x1*x2, x*x2, x*x1]
    } else if n == 2 {
        vec![x-F::from(1), x]
    } else {panic!()};

    ret.iter().enumerate().map(|(i, val)| *val*F::inv_lagrange_prod(i as u64, n)).collect()
}

/// Gadget which takes as an input n vector variables, and an index variable, and returns a variable #i.
pub fn choice_gadget<'a, F: PrimeField+FieldUtils> (
        circuit: &mut Circuit<'a, F, Gatebb<'a,F>>,
        variants: &[&[Variable]],
        index: VarRange,
        round: usize) -> Vec<Variable> {

    let n = index.range;
    assert!(BigUint::from(variants.len()) == n);
    let n = variants.len();
    let q = variants[0].len();
    for v in variants {
        assert!(v.len() == q);
    }

    let v : Vec<_> = variants.iter().map(|x|*x).flatten().map(|x|*x).chain([index.var].into_iter()).collect();
    
    let choice_poly = PolyOp::new(
        n,
        n*q+1,
        q,
        move |args, _| {
            let (variants, index) = args.split_at(n*q);
            let index = index[0];
            let choice_coeffs = lagrange_choice_batched(index, n as u64);
            let mut ret : Vec<_> = repeat(F::ZERO).take(q).collect();
            for i in 0..n {
                for j in 0..q {
                    ret[j] += variants[i*q + j]*choice_coeffs[i]
                }
            }
            ret
        }
    );

    circuit.apply(round, choice_poly, v)
}



pub fn limb_decompose_no_lookup_gadget<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
    base: u32,
    round: usize,
    num_limbs: usize,
    input: Variable
) -> Vec<VarRange> {
    limb_decompose_unchecked(circuit, base, round, num_limbs, input)
        .iter().map(|var|VarRange::new_no_lookup(circuit, *var, base)).collect()
        // Note that this constrains limbs to be limbs.
}