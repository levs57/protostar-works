use std::{rc::Rc, iter::repeat, marker::PhantomData};

use ff::PrimeField;
use num_bigint::BigUint;

use crate::{constraint_system::Variable, utils::{field_precomp::FieldUtils, arith_helper::modulus}, circuit::{Circuit, Advice}, gate::Gatebb, gadgets::rangecheck_small::rangecheck};

use super::{rangecheck_lookup::{RangeLookup, self, limb_decompose_with_lookup_gadget}, lookup::Lookup};


#[derive(Clone)]
/// Range-checked variable of limb-size.
pub struct VarRange<F: PrimeField+FieldUtils> {
    pub var: Variable,
    pub range: BigUint,
    _marker: PhantomData<F>,
}

impl<F:PrimeField+FieldUtils> VarRange<F> {
    
    /// Believes that variable var is in range. Demands that the range is less than field modulus,
    /// all implementations that create VarRange must go through this check.
    pub fn new_unchecked(var: Variable, range: BigUint) -> Self {
        assert!(range < modulus::<F>(), "Construction error: variable does not fit in a single field element.");
        Self { var, range, _marker: PhantomData::<F> }
    }

    /// Range-checks variable var. Base = limb size. Do not use for large base.
    pub fn new_no_lookup<'a>(
        circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
        var: Variable,
        base: u32) -> Self {
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
        checker: &mut RangeLookup<F>) -> Self {
            checker.check(circuit, var);
            Self::new_unchecked(var, BigUint::from(checker.range()))
    }


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