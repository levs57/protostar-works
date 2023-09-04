use std::{iter::repeat, cmp::max};

use ff::PrimeField;
use num_traits::{Pow, pow};
use rand_core::OsRng;

use crate::{witness::CSWtns, gate::{Gate, Gatebb}, constraint_system::Variable};

pub enum ExecMode{
    Constrain,
    Execute,
}

/// A polynomial operation. First argument is a relaxation factor, passed separately.
/// Must be homogeneous (current API limitation, need to remove it at some point).
/// In a gate, it is by convention transformed to be first input.
pub struct PolyOp<'a, F:PrimeField>{
    pub d: usize,
    pub i: usize,
    pub o: usize,
    pub f: Box<dyn Fn(F, &[F]) -> Vec<F> + 'a>,
}

impl<'a, F:PrimeField> PolyOp<'a, F> {
    pub fn new(d: usize, i: usize, o: usize, f: Box<dyn Fn(F, &[F]) -> Vec<F> + 'a>) -> Self{
        let random_input : (_, Vec<_>) = (F::random(OsRng) , repeat(F::random(OsRng)).take(i).collect()); 
        let random_input_2 : (_, Vec<_>) = (random_input.0*F::from(2), random_input.1.iter().map(|x| *x*F::from(2)).collect());
        assert!({
            let mut flag = true;
            (&f)(random_input_2.0, &random_input_2.1).iter().zip((&f)(random_input.0, &random_input.1).iter())
                .map(|(a, b)| {
                    flag &= (*a==*b*F::from(pow(2, d)))
                }).count();
            flag
        }, "Sanity check failed - provided f is not a polynomial of degree d");
 
        PolyOp { d, i, o, f }
    }

    pub fn into_gate(&'a self) -> Gatebb<'a, F> {
        let d = self.d;
        let i = self.i + 1 + self.o;
        let o = self.o;

        let f = |args: &[F]| {
            let (inputs, outputs) = args.split_at(self.i);
            let (one, inputs) = inputs.split_at(1);
            let one = one[0];
            let results = (self.f)(one, &inputs);
            let onepow = one.pow([(self.d-1) as u64]);
            results.iter().zip(outputs.iter()).map(|(inp, out)|*inp-*out*onepow).collect()
        };

        Gatebb::new(d, i, o, Box::new(f))
    }
}

pub struct Circuit<'a, F: PrimeField> {
    pub cs: CSWtns<'a, F>,
    pub ops: Vec<Box<dyn 'a + Fn(&mut CSWtns<'a, F>) -> ()>>,
    pub mode: ExecMode,
}

impl<'a, F:PrimeField> Circuit<'a, F>{
    pub fn apply(&mut self, op: PolyOp<'a, F>, args: &[Variable]) -> (){
        
    }
}

