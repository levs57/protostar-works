use std::{rc::Rc, iter::repeat, marker::PhantomData};

use ff::PrimeField;
use itertools::Itertools;
use num_bigint::BigUint;
use num_traits::FromPrimitive;

use crate::{constraint_system::Variable, utils::{field_precomp::FieldUtils, arith_helper::{modulus, from_biguint}}, circuit::{Circuit, Advice}, gate::Gatebb, gadgets::rangecheck_small::rangecheck};

use super::{rangecheck_lookup::{RangeLookup, self, limb_decompose_with_lookup_gadget}, lookup::Lookup, lc::{lc_constr, lc}, arith::mul_gadget};


#[derive(Clone)]
/// Range-checked variable of limb-size.
pub struct VarRange<F: PrimeField+FieldUtils> {
    var: Variable,
    range: BigUint,
    _marker: PhantomData<F>,
}

impl<F:PrimeField+FieldUtils> VarRange<F> {
    
    pub fn range(&self) -> BigUint {
        self.range.clone()
    }

    pub fn var(&self) -> Variable {
        self.var
    }

    /// Believes that variable var is in range. Demands that the range is less than field modulus,
    /// all implementations that create VarRange must go through this check.
    pub fn new_unchecked(var: Variable, range: BigUint) -> Self {
        assert!(range != BigUint::from(0u8), "Construction error: range must be positive.");
        assert!(range <= modulus::<F>(), "Construction error: variable does not fit in a single field element.");
        Self { var, range, _marker: PhantomData::<F> }
    }

    pub fn from_var(var: Variable) -> Self {
        Self::new_unchecked(var, modulus::<F>())
    }

    /// Range-checks variable var. Base = limb size. Do not use for large base.
    pub fn new_no_lookup<'a>(
        circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
        var: Variable,
        base: u32,
    ) -> Self {
        circuit.constrain(&[var], Gatebb::new(base as usize, 1, 1,
            Rc::new(move |args, _|{
                vec![rangecheck(args[0], base as u64)]
            }), 
            vec![],
        ));

        Self::new_unchecked(var, BigUint::from(base))
    }

    pub fn new_with_lookup<'a>(
        circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
        var: Variable,
        checker: &mut RangeLookup<F>,
    ) -> Self {
        checker.check(circuit, var);
        Self::new_unchecked(var, BigUint::from(checker.range()))
    }

    pub fn upscale(&mut self, new_range: &BigUint) -> () {
        assert!(*new_range >= self.range);
        *self = Self::new_unchecked(self.var(), new_range.clone());
    }
}

pub fn lc_uint_constrain<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
    coeffs: &[BigUint],
    vars: &[VarRange<F>],
    sum: Variable,
) -> VarRange<F> {
    let range = coeffs.iter()
        .zip_eq(vars.iter().map(|x|x.range()-BigUint::from(1u8)))
        .fold(BigUint::from(1u8), |acc, (x,y)|acc+x*y);
    let mut coeffs = coeffs.iter().map(|x|from_biguint::<F>(x)).collect_vec();
    let mut vars = vars.iter().map(|x|x.var()).collect_vec();

    coeffs.push(-F::ONE);
    vars.push(sum);
    lc_constr(circuit, &coeffs, &vars);

    VarRange::new_unchecked(sum, range)
}

pub fn lc_uint<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
    coeffs: &[BigUint],
    vars: &[VarRange<F>],
    round: usize,
) -> VarRange<F> {
    let range = coeffs.iter()
        .zip_eq(vars.iter().map(|x|x.range()-BigUint::from(1u8)))
        .fold(BigUint::from(1u8), |acc, (x,y)|acc+x*y);
    let coeffs = coeffs.iter().map(|x|from_biguint::<F>(x)).collect_vec();
    let vars = vars.iter().map(|x|x.var()).collect_vec();
    let ret = lc(circuit, &coeffs, &vars, round);

    VarRange::new_unchecked(ret, range)
}

pub fn mul_uint<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
    a: &VarRange<F>,
    b: &VarRange<F>,
    round: usize,
) -> VarRange<F> {
    let prod = mul_gadget(circuit, a.var(), b.var(), round);
    let range = (a.range() - BigUint::from(1u8)) * (b.range() - BigUint::from(1u8)) + BigUint::from(1u8);

    VarRange::new_unchecked(prod, range)
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

pub fn limb_decompose_unchecked<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
    base: u32,
    round: usize,
    num_limbs: usize,
    input: Variable
) -> Vec<Variable> {
    let mut limbs = circuit.advice(
        round,
        Advice::new(
            1,
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
    
    limbs
}

/// Convenience function - constructs a range-checked value from limbs using a nonnegative linear combination.
/// Requires a set of bases as a bug prevention measure - will upscale ranges to these bases.
pub fn from_limbs<'a, F: PrimeField+FieldUtils> (
    circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
    limbs: &[VarRange<F>],
    bases: &[BigUint],
    round: usize,
) -> VarRange<F> {
    let mut coeffs = vec![];
    let mut limbs = limbs.to_vec();
    limbs.iter_mut().zip_eq(bases.into_iter()).map(|(l, b)| {
        l.upscale(b);
    }).count();
    let mut tmp = BigUint::from(1u8);
    for i in 0..limbs.len() {
        coeffs.push(tmp.clone());
        tmp *= limbs[i].range();
    }
    lc_uint(circuit, &coeffs, &limbs, round)
}