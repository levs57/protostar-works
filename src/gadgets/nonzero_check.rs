use std::{cmp::max};
use ff::PrimeField;
use crate::{utils::field_precomp::FieldUtils, circuit::{Circuit, Advice}, gate::Gatebb, constraint_system::Variable};
use super::running_prod::prod_run_gadget;
use crate::gatelib::nonzero_check;


/// Checks that the array of variables is nonzero.
/// Rate = amount of elements processed in a single chunk.
pub fn nonzero_gadget<'a, F: PrimeField + FieldUtils> (circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, input: &[Variable], rate: usize) -> () {
    let mut round = 0;
    for v in input {
        round = max(
            round,
            v.round
        )
    }
    
    let prod = prod_run_gadget(circuit, input.to_vec(), round, rate);
    let adv_invert = Advice::new(1, 0, 1, |arg: &[F], _| vec![arg[0].invert().unwrap()]);

    let prod_inv = circuit.advice(round, adv_invert, vec![prod], vec![])[0];

    circuit.constrain_with(
        &vec![prod, prod_inv], 
        &[],
        &nonzero_check(),
    );
}

// pub struct NonzeroSubroutine<'a, F: PrimeField+FieldUtils> {
//     subroutine: SubroutineDefault<'a, F, usize, (), Gatebb<'a, F>>
// }

// impl<'a, F: PrimeField+FieldUtils> Subroutine<'a, F, usize, (), Gatebb<'a, F>> for NonzeroSubroutine<'a, F> {

//     type InitParams = ();

//     fn new(circuit: &'a mut Circuit<'a, F, Gatebb<'a, F>>, dummy_value: &'a ExternalValue<F>, params: usize, _:()) -> Self {
//         let subroutine = SubroutineDefault::new(circuit, dummy_value, params, nonzero_gadget);
//         Self { subroutine }
//     }
//     fn push (&mut self, v: Variable) -> () {
//         self.subroutine.push(v);
//     }
//     fn finalize (&'a mut self, circuit: &mut Circuit<'a, F,Gatebb<'a, F>>) -> () {
//         self.subroutine.finalize(circuit);
//     }
// }

