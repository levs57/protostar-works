use std::rc::Rc;
use ff::PrimeField;
use crate::{circuit::{Circuit, Build}, utils::field_precomp::FieldUtils, gate::Gatebb, constraint_system::Variable};

pub fn eq_gadget<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a,F,Gatebb<'a,F>,Build>,
    a: Variable,
    b: Variable,
) -> () {
    circuit.constrain(&vec![a,b], Gatebb::new(1, 2, 1, Rc::new(|args|vec![args[0]-args[1]])));
}