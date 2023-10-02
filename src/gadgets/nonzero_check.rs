use std::{rc::Rc, cmp::max};

use ff::PrimeField;

use crate::{utils::field_precomp::FieldUtils, circuit::{Circuit, Advice, ExternalValue}, gate::Gatebb, constraint_system::Variable, subroutine::{SubroutineDefault, Subroutine}};

use super::running_prod::prod_run_gadget;

/// Checks that the array of variables is nonzero.
/// Rate = amount of elements processed in a single chunk.
fn nonzero_gadget<'a, F: PrimeField + FieldUtils> (circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, input: &[Variable], rate: &usize) -> () {
    let mut round = 0;
    for v in input {
        round = max(
            round,
            match v {
                Variable::Public(_, r) => *r,
                Variable::Private(_, r) => *r,
            }
        )
    }
    
    let prod = prod_run_gadget(circuit, input.to_vec(), round, *rate);
    let adv_invert = Advice::new(1, 0, 1,
        Rc::new(|arg: &[F], _| vec![arg[0].invert().unwrap()])
    );

    let prod_inv = circuit.advice(round, adv_invert, vec![prod], vec![])[0];

    circuit.constrain(
        &vec![prod, prod_inv], 
        Gatebb::new(2, 2, 1,
            Rc::new(|args|vec![args[0]*args[1] - F::ONE]))
    );
}

pub struct NonzeroSubroutine<'a, F: PrimeField+FieldUtils> {
    subroutine: SubroutineDefault<'a, F, usize, (), Gatebb<'a, F>>
}

impl<'a, F: PrimeField+FieldUtils> Subroutine<'a, F, usize, (), Gatebb<'a, F>> for NonzeroSubroutine<'a, F> {
    fn new(params: usize) -> Self {
        Self { subroutine: 
            SubroutineDefault {
                seal: ExternalValue::new(),
                vars: vec![],
                params,
                gadget: nonzero_gadget,
            }
        }
    }
    fn push (&mut self, v: Variable) -> () {
        self.subroutine.push(v);
    }
    fn finalize (&'a mut self, circuit: &mut Circuit<'a, F,Gatebb<'a, F>>) -> () {
        self.subroutine.finalize(circuit);
    }
}

