// Utils for nonnative arithmetic used in folding.

use std::marker::PhantomData;
use ff::PrimeField;
use halo2::halo2curves::bn256;
use itertools::Itertools;
use num_bigint::BigUint;
use crate::{constraint_system::Variable, utils::{field_precomp::FieldUtils, arith_helper::modulus}, gate::Gatebb, circuit::Circuit, folding::poseidon::Poseidon};

use super::{rangecheck_common::{VarRange, from_limbs}, poseidon::poseidon_gadget};

type F = bn256::Fr;
pub struct RangeAwareHasher<F: PrimeField+FieldUtils> {
    values: Vec<VarRange<F>>,
}

impl RangeAwareHasher<F> {
    pub fn new() -> Self {
        Self {values: vec![]}
    }

    pub fn consume(&mut self, v: VarRange<F>) {
        self.values.push(v)
    }

    pub fn hash<'a>(self, circuit: &mut Circuit<'a, F, Gatebb<'a,F>>, cfg: &'a Poseidon, rate: usize, round: usize) -> Variable {
        let mut limbs = vec![];
        let mut total_range = BigUint::from(1u8);
        let mut combined = vec![];
        for value in self.values {
            let tmp = total_range*value.range();
            if tmp <= modulus::<F>() {
                limbs.push(value);
                total_range = tmp;
            } else {
                let bases = limbs.iter().map(|x|x.range()).collect_vec();
                combined.push(from_limbs(circuit, &limbs, &bases, round).var());
                limbs = vec![value.clone()];
                total_range = value.range();
            }
        }
        let bases = limbs.iter().map(|x|x.range()).collect_vec();
        if limbs.len() > 0 {
            combined.push(from_limbs(circuit, &limbs, &bases, round).var())
        }
        poseidon_gadget(circuit, cfg, round, rate, &combined)
    }
}


