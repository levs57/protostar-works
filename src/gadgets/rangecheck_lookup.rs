use ff::{PrimeField};
use itertools::Itertools;
use num_bigint::BigUint;
use crate::{constraint_system::Variable, circuit::{Circuit, ExternalValue}, gate::Gatebb, utils::field_precomp::FieldUtils};

use super::{lookup::{StaticLookup, Lookup}, rangecheck_common::RangeCheckedVariable};

pub struct VarRLookup {
    var: Variable,
    range: BigUint
}

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

impl<F: PrimeField+FieldUtils> RangeCheckedVariable<BigUint, RangeLookup<F>, F> for VarRLookup {
    fn new_unchecked(var: Variable, range: BigUint) -> Self {
        todo!()
    }
    fn new<'a>(
            circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
            var: Variable,
            base: u32,
            checker: &mut RangeLookup<F>) -> Self {
        todo!()
    }
}