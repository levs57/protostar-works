// Arbitrary subroutine of the circuit that must be finalized before finalizing it.
// Useful for batch-processing in cases where we can not predict how many batches will be present.

// Basically, these are autonomous sub-circuits which can be fed arbitrary amount of data and know what to do with it.
// Most useful are probably lookup subroutines and inversion subroutines.

use std::rc::Rc;

use ff::PrimeField;
use crate::{circuit::{ExternalValue, Circuit, PolyOp, Advice}, constraint_system::Variable, gate::Gate, utils::field_precomp::FieldUtils};

pub struct SubroutineDefault<'a, F: PrimeField + FieldUtils, Params, Output, T: Gate<F> + From<PolyOp<'a, F>>,
> {
    pub(crate) seal: ExternalValue<F>,
    pub(crate) vars: Vec<Variable>,
    pub(crate) params: Params,
    pub(crate) gadget: fn(&mut Circuit<'a, F, T>, &[Variable], &Params) -> Output,
}

pub trait Subroutine<'a, F: PrimeField + FieldUtils, Params, Output, T: Gate<F> + From<PolyOp<'a, F>>> {
    fn new(params: Params) -> Self;
    fn push (&mut self, v: Variable) -> ();
    fn finalize (&'a mut self, circuit: &mut Circuit<'a, F,T>) -> Output;
}


impl<'a, F: PrimeField + FieldUtils, Params, Output, T: Gate<F> + From<PolyOp<'a, F>>> Subroutine <'a, F, Params, Output, T> for SubroutineDefault<'a, F, Params, Output, T> {
    fn new (params: Params) -> Self {
        unreachable!()
    }

    fn push (&mut self, v: Variable) -> () {
        match self.seal.get() {
            None => self.vars.push(v),
            Some(_) => panic!(),
        }
    }

    fn finalize (&'a mut self, circuit: &mut Circuit<'a, F,T>) -> Output {
        let adv = Advice::new(
            0,
            1,
            0,
            Rc::new(move |_,_|vec![])
        );

        circuit.advice(    // We give an empty advice which attempts to read seal. If it is empty, this will fail.
            0,
            adv,
            vec![],
            vec![&self.seal]
        );
        (self.gadget)(circuit, &self.vars, &self.params)
    }
}