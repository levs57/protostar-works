use ff::{PrimeField};
use itertools::Itertools;
use num_bigint::BigUint;
use crate::{constraint_system::Variable, circuit::{Circuit, ExternalValue}, gate::Gatebb, utils::field_precomp::FieldUtils};

use super::{lookup::{StaticLookup, Lookup}, rangecheck_common::{limb_decompose_unchecked, VarRange}};

pub struct RangeLookup<F: PrimeField+FieldUtils> {
    lookup: StaticLookup<F>,
    rangetable: Vec<F>,
}

impl<F: PrimeField+FieldUtils> RangeLookup<F> {
    pub fn new(challenge_src: ExternalValue<F>, range: usize) -> Self {
        let rangetable = (0..range).map(|x|F::from(x as u64)).collect_vec();
        let lookup = StaticLookup::new(challenge_src, &rangetable);
        Self {lookup, rangetable}
    }

    pub fn range(&self) -> usize {
        self.rangetable.len()
    }
}

impl<'a, F: PrimeField+FieldUtils> Lookup<'a, F> for RangeLookup<F> {
    fn check(&mut self, circuit: &mut Circuit<'a, F, Gatebb<'a,F>>, var: Variable) -> () {
        self.lookup.check(circuit, var);
    }
    fn finalize(
            self,
            circuit: &mut Circuit<'a, F, Gatebb<'a,F>>,
            table_round: usize,
            access_round: usize,
            challenge_round: usize,
            rate: usize,
        ) -> () {
        self.lookup.finalize(circuit, table_round, access_round, challenge_round, rate)
    }
}

pub fn limb_decompose_with_lookup_gadget<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
    round: usize,
    num_limbs: usize,
    checker: &mut RangeLookup<F>,
    input: Variable
) -> Vec<VarRange<F>> {
    limb_decompose_unchecked(circuit, checker.range() as u32, round, num_limbs, input)
        .iter().map(|var|VarRange::new_with_lookup(circuit, *var, checker)).collect()
        // Note that this constrains limbs to be limbs.
}