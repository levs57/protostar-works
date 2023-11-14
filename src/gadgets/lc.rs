// This gadget implements linear combination. At some point it should be deprecated; now we will use it to safely
// wrap every instance of large linear combination.

use std::rc::Rc;

use ff::PrimeField;
use itertools::Itertools;
use crate::{utils::field_precomp::FieldUtils, circuit::{Circuit, Build, Advice, PolyOp}, gate::Gatebb, constraint_system::Variable};

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
    let poly = PolyOp::new(2, 2*l, 1, split_and_ip::<F>);
    let args : Vec<_> = a.iter().chain(b.iter()).map(|x|*x).collect();
    circuit.apply(round, poly, args)[0]
}

pub fn lc<'a, F: PrimeField+FieldUtils>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>, Build>, coeffs:&'a [F], vars: &[Variable], round: usize) -> Variable {
    assert_eq!(coeffs.len(), vars.len());
    let l = vars.len();
    let poly = PolyOp::new(1, l, 1, |args|{inner_prod(coeffs, args)}); // NO MOVE HERE!!
    circuit.apply(round, poly, vars.to_vec())[0]
}