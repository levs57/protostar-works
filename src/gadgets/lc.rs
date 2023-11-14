// This gadget implements linear combination. At some point it should be deprecated; now we will use it to safely
// wrap every instance of large linear combination.

use std::rc::Rc;

use ff::PrimeField;
use itertools::Itertools;
use crate::{utils::field_precomp::FieldUtils, circuit::{Circuit, Build, Advice}, gate::Gatebb, constraint_system::Variable};

fn inner_prod<F: PrimeField+FieldUtils>(a: &[F], b: &[F]) -> Vec<F> {
    vec![a.iter().zip_eq(b.iter()).fold(F::ZERO, |acc, (x,y)|acc+*x*y)]
}

fn split_and_ip<F: PrimeField+FieldUtils>(args: &[F]) -> Vec<F> {
    assert!(args.len()%2 == 0);
    let (a, b) = args.split_at(args.len()/2);
    inner_prod(a, b)
}

fn ip_out<F: PrimeField+FieldUtils>(a: &[F], b: &[F]) -> Vec<F> {
    assert!(a.len()+1 == b.len());
    vec![a.iter().zip(b.iter()).fold(F::ZERO, |acc, (x,y)|acc+*x*y) - b[b.len()-1]]
}

fn split_and_ip_out<F: PrimeField+FieldUtils>(args: &[F]) -> Vec<F> {
    assert!(args.len()%2 == 1);
    let (a, b) = args.split_at(args.len()/2);
    ip_out(a, b)
}

/// Linear combination with constant coefficients. Constrain version.
pub fn lc_constr<'a, F: PrimeField+FieldUtils>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>, Build>, coeffs:&'a [F], vars: &[Variable]) -> () {
    assert_eq!(coeffs.len(), vars.len());
    let l = vars.len();
    let gate = Gatebb::new(1, l, 1, Rc::new(|args|{inner_prod(coeffs, args)})); // NO MOVE HERE!!
    circuit.constrain(vars, gate);
}

pub fn qc<'a, F: PrimeField+FieldUtils>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>, Build>, a: &[Variable], b: &[Variable], round: usize) -> Variable {
    assert_eq!(a.len(), b.len());
    let l = a.len();
    let gate = Gatebb::new(2, 2*l+1, 1, Rc::new(split_and_ip_out::<F>));
    let advice = Advice::new(2*l, 0, 1, |args, _:&[F]|{split_and_ip(args)});
    let mut args : Vec<_> = a.iter().chain(b.iter()).map(|x|*x).collect();
    let output = circuit.advice(round, advice, args.clone(), vec![])[0];
    args.push(output);
    circuit.constrain(&args, gate);
    output
}

pub fn lc<'a, F: PrimeField+FieldUtils>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>, Build>, coeffs:&'a [F], vars: &[Variable], round: usize) -> Variable {
    assert_eq!(coeffs.len(), vars.len());
    let l = vars.len();
    let gate = Gatebb::new(1, l+1, 1, Rc::new(|args|{ip_out(coeffs, args)})); // NO MOVE HERE!!
    let advice = Advice::new(l, 0, 1, |args, _:&[F]|{inner_prod(coeffs, args)});
    let output = circuit.advice(round, advice, vars.to_vec(), vec![])[0];
    let mut vars = vars.to_vec();
    vars.push(output);
    circuit.constrain(&vars, gate);
    output
}

