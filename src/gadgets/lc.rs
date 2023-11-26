
// This gadget implements linear combination. At some point it should be deprecated; now we will use it to safely
// wrap every instance of large linear combination.

use std::rc::Rc;

use ff::PrimeField;
use itertools::Itertools;
use crate::{utils::field_precomp::FieldUtils, circuit::{Circuit, PolyOp}, gate::Gatebb, constraint_system::Variable};

pub fn inner_prod<F: PrimeField+FieldUtils>(a: &[F], b: &[F]) -> F {
    a.iter().zip_eq(b.iter()).fold(F::ZERO, |acc, (x,y)|acc+*x*y)
}

fn split_and_ip<F: PrimeField+FieldUtils>(args: &[F]) -> Vec<F> {
    assert!(args.len()%2 == 0);
    let (a, b) = args.split_at(args.len()/2);
    vec![inner_prod(a, b)]
}

pub fn sum_arr<F: PrimeField+FieldUtils>(args: &[F]) -> F {
    args.iter().fold(F::ZERO, |acc, upd| acc+upd)
}

/// Linear combination with constant coefficients. Constrain version.
pub fn lc_constr<'a, F: PrimeField+FieldUtils>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, coeffs:&[F], vars: &[Variable]) -> () {
    assert_eq!(coeffs.len(), vars.len());
    let l = vars.len();
    let gate = Gatebb::new(1, l, 1, Rc::new(|args, coeffs|{vec![inner_prod(coeffs, args)]}), vec![]); // NO MOVE HERE!!
    circuit.constrain(vars, gate);
}

pub fn qc<'a, F: PrimeField+FieldUtils>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, a: &[Variable], b: &[Variable], round: usize) -> Variable {
    assert_eq!(a.len(), b.len());
    let l = a.len();
    let poly = PolyOp::new(2, 2*l, 1, |args, _| split_and_ip::<F>(args));
    let args : Vec<_> = a.iter().chain(b.iter()).map(|x|*x).collect();
    circuit.apply(round, poly, args)[0]
}

pub fn lc<'a, F: PrimeField+FieldUtils>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, coeffs:&[F], vars: &[Variable], round: usize) -> Variable {
    assert_eq!(coeffs.len(), vars.len());
    let l = vars.len();
    let poly = PolyOp::new(1, l, 1, |args, coeffs|{vec![inner_prod(coeffs, args)]});
    circuit.apply(round, poly, vars.to_vec())[0]
}

pub fn sum_gadget<'a, F: PrimeField+FieldUtils>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, vars: &[Variable], round: usize) -> Variable {
    let l = vars.len();
    let poly = PolyOp::new(1, l, 1, |arr, _|vec![sum_arr(arr)]);
    circuit.apply(round, poly, vars.to_vec())[0]
}