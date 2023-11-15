use ff::PrimeField;

use crate::{circuit::{Circuit, Build, ExternalValue, Advice, AdvicePub}, utils::field_precomp::FieldUtils, gate::Gatebb, constraint_system::Variable};

pub fn input<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a,F,Gatebb<'a,F>,Build>,
    inp: &'a ExternalValue<F>,
    round: usize,
) -> Variable {
    circuit.advice_pub(round, AdvicePub::new(1, 1, |arg|arg.to_vec()), vec![inp])[0]
}