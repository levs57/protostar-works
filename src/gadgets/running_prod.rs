// Nonzero elements checker.

use std::rc::Rc;
use ff::PrimeField;
use crate::{utils::field_precomp::FieldUtils, circuit::{Circuit, PolyOp}, gate::Gatebb, constraint_system::Variable};


/// Outputs the product of an array in a single polynomial.
pub fn prod_flat_gadget<'a, F: PrimeField + FieldUtils> (circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, input: Vec<Variable>, round: usize) -> Variable {
    let l = input.len();
    if l == 0 {return circuit.one()} // product of 0 elements is 1
    if l == 1 {return input[0]}
    let prod_all = PolyOp::new(l, l, 1,
        Rc::new(
            |args| vec![args.iter().fold(F::ONE, |acc, upd| acc*upd)]
        )
    );
    circuit.apply(round, prod_all, input)[0]
}

/// Outputs the product of an array, multiplying them in rate -sized chunks.
pub fn prod_run_gadget<'a, F: PrimeField + FieldUtils> (circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, input: Vec<Variable>, round: usize, rate: usize) -> Variable {
    assert!(rate>0);
    let mut acc = vec![];
    for i in 0..input.len() {
        if acc.len() == rate {
            acc = vec![prod_flat_gadget(circuit, acc, round)];
        }
        acc.push(input[i]);
    }
    prod_flat_gadget(circuit, acc, round)
}