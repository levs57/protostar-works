use std::rc::Rc;
use ff::PrimeField;
use gate_macro::make_gate;
use crate::{circuit::{Circuit, Advice}, utils::field_precomp::FieldUtils, gate::Gatebb, constraint_system::Variable};
use elsa::FrozenMap;

pub fn eq_gadget<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a,F,Gatebb<'a,F>>,
    a: Variable,
    b: Variable,
) -> () {
    circuit.constrain(&vec![a,b], Gatebb::new(1, 2, 1, Rc::new(|args, _|vec![args[0]-args[1]]), vec![]));
}

#[make_gate]
pub fn arith_gate<'c, F: PrimeField>(smul: F, sa: F, sb: F, sconst: F)->Gatebb<'c, F>{
    Gatebb::new(2, 3, 1, Rc::new(|args, consts|{
        let a = args[0];
        let b = args[1];
        let c = args[2];
        let smul = consts[0];
        let sa = consts[1];
        let sb = consts[2];
        let sconst = consts[3];
        vec![smul*a*b + sa*a + sb*b + sconst - c]
    }), vec![smul, sa, sb, sconst])
}

pub fn arith_gadget<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a,F,Gatebb<'a,F>>,
    a: Variable,
    b: Variable,
    smul: F,
    sa: F,
    sb: F,
    sconst: F,
    round: usize,
) -> Variable {
    let advice = Advice::new(2, 1, move |args, _|{
        let a = args[0];
        let b = args[1];
        vec![smul*a*b + sa*a + sb*b + sconst]
    });
    let c = circuit.advice(round, advice, vec![a,b])[0];
    circuit.constrain_with(&vec![a,b,c], &arith_gate(smul, sa, sb, sconst));
    c
}

pub fn add_gadget<'a, F: PrimeField+FieldUtils> (
    circuit: &mut Circuit<'a,F,Gatebb<'a,F>>,
    a: Variable,
    b: Variable,
    round: usize,
) -> Variable {
    arith_gadget(circuit, a, b, F::ZERO, F::ONE, F::ONE, F::ZERO, round)
}

pub fn mul_gadget<'a, F: PrimeField+FieldUtils> (
    circuit: &mut Circuit<'a,F,Gatebb<'a,F>>,
    a: Variable,
    b: Variable,
    round: usize,
) -> Variable {
    arith_gadget(circuit, a, b, F::ONE, F::ZERO, F::ZERO, F::ZERO, round)
}