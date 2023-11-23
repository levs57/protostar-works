use std::rc::Rc;
use ff::PrimeField;
use gate_macro::make_gate;
use itertools::Itertools;
use crate::{circuit::{Circuit, Advice}, utils::field_precomp::FieldUtils, gate::Gatebb, constraint_system::Variable};
use elsa::FrozenMap;


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

#[make_gate]
pub fn read_const_gate<'c, F: PrimeField>(c: F)->Gatebb<'c, F>{
    Gatebb::new(1, 1, 1, Rc::new(|args, consts|{
        let a = args[0];
        let c = consts[0];
        vec![a-c]
    }), vec![c])
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

pub fn eq_gadget<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a,F,Gatebb<'a,F>>,
    a: Variable,
    b: Variable,
) -> () {
    let dummy = a;
    circuit.constrain_with(&vec![a, dummy, b], &arith_gate(F::ZERO, F::ONE, F::ZERO, F::ZERO));
}


pub fn read_const_gadget<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a,F,Gatebb<'a,F>>,
    c: F,
    round: usize,
) -> Variable {
    let advice = Advice::new(0, 1, move |_, _| vec![c]);
    let v = circuit.advice(round, advice, vec![])[0];
    circuit.constrain_with(&vec![v], &read_const_gate(c));
    v
}
