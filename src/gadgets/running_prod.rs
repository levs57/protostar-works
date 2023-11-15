// Nonzero elements checker.

use ff::PrimeField;
use crate::{utils::field_precomp::FieldUtils, circuit::{Circuit, PolyOp}, gate::Gatebb, constraint_system::Variable};


/// Outputs the product of an array in a single polynomial.
pub fn prod_flat_gadget<'a, F: PrimeField + FieldUtils> (circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, input: Vec<Variable>, round: usize) -> Variable {
    match input.len() {
        0 => circuit.one(),  // product of 0 elements is 1
        1 => *input.first().expect("should not be empty"),
        n => {
            let prod = PolyOp::new(n, n, 1, |args| vec![args.iter().product()]);
            circuit.apply(round, prod, input)[0]
        }
    }
}

/// Outputs the product of an array, multiplying them in rate -sized chunks.
pub fn prod_run_gadget<'a, F: PrimeField + FieldUtils> (circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, input: Vec<Variable>, round: usize, rate: usize) -> Variable {
    assert!(rate > 0);

    // first `rate` elems are processed together,
    // the rest are taken `rate - 1` at a time and processed with
    // the previous result
    let mut acc = vec![];
    for i in 0..input.len() {
        if acc.len() == rate {
            acc = vec![prod_flat_gadget(circuit, acc, round)];
        }
        acc.push(input[i]);
    }
    prod_flat_gadget(circuit, acc, round)
}