use std::{rc::Rc, iter::repeat};

use ff::PrimeField;
use crate::{utils::field_precomp::FieldUtils, gate::Gatebb, circuit::{Circuit, Advice, PolyOp}, constraint_system::Variable};
use num_bigint::{self, BigUint};

#[derive(Clone, Copy)]
/// Range-checked variable of limb-size.
pub struct VarSmall {
    pub var: Variable,
    pub range: u32,
}

impl VarSmall {
    
    /// Believes that variable var it is in range.
    pub fn new_unchecked(var: Variable, range: u32) -> Self {
        Self { var, range }
    }

    /// Range-checks variable var. Base = limb size.
    pub fn new<'a, F: PrimeField+FieldUtils>(
        circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
        var: Variable,
        base: u32) -> Self {

        circuit.constrain(&[var], Gatebb::new(base as usize, 1, 1,
            Rc::new(move |args, _|{
                vec![rangecheck(args[0], base as u64)]
            }), 
            vec![],
        ));

        Self::new_unchecked(var, base)

    }
}

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

pub fn limbs<F: PrimeField>(x: F, base: u32) ->  Vec<u32>{
    let mut x = BigUint::from_bytes_le(x.to_repr().as_ref());
    let mut ret = vec![];
    loop {
        let y = x.clone()%base;
        x = x/base;
        ret.push(y.to_u32_digits()[0]);
        if x==BigUint::from(0 as u64) {break}
    }
    ret
}

pub fn limb_decompose_gadget<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
    base: u32,
    round: usize,
    num_limbs: usize,
    input: Variable
) -> Vec<VarSmall> {
    let mut limbs = circuit.advice(
        round,
        Advice::new(
            1,
            0,
            num_limbs,
            move |args, _| {
                let x = args[0];
                let limbs = limbs(x, base);
                assert!(limbs.len()<=num_limbs, "The value has too many limbs.");
                limbs.into_iter().map(|x|F::from(x as u64)).chain(repeat(F::ZERO)).take(num_limbs).collect()
            }
        ),
        vec![input],
    );


    limbs.push(input);

    circuit.constrain(&limbs, Gatebb::new(1, num_limbs+1, 1,
            Rc::new(move |args, _| {
                let mut acc = F::ZERO;
                for i in 0..num_limbs {
                    acc = acc.scale(base as u64);
                    acc += args[num_limbs-i-1];
                }
                vec![acc - args[num_limbs]]
            }), 
            vec![],
        )
    );

    limbs.pop();
    
    limbs.iter().map(|var|VarSmall::new(circuit, *var, base)).collect() // Note that this constrains limbs to be limbs.

}

/// Gadget which takes as an input n vector variables, and an index variable, and returns a variable #i.
pub fn choice_gadget<'a, F: PrimeField+FieldUtils> (
        circuit: &mut Circuit<'a, F, Gatebb<'a,F>>,
        variants: &[&[Variable]],
        index: VarSmall,
        round: usize) -> Vec<Variable> {

    let n = index.range as usize;
    assert!(variants.len() == n);
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