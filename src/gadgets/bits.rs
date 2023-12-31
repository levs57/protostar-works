// Bit decomposition gadget

use std::{rc::Rc, iter::repeat};

use ff::PrimeField;

use crate::{circuit::{Circuit, Advice}, gate::Gatebb, constraint_system::Variable};
use crate::utils::field_precomp::FieldUtils;

pub fn bitcheck<F: PrimeField>(arg: &[F]) -> Vec<F> {
    let x = arg[0];
    vec![x*x - x]
}

pub fn decompcheck<F: PrimeField>(arg: &[F]) -> Vec<F> {
    
    let x = arg[0];
    let mut acc = F::ZERO;
    for i in 1..arg.len() {
        acc += acc;
        acc += arg[arg.len()-i];
    }
    vec![acc-x]
}

pub fn bit_decomposition_gadget<'a, F: PrimeField+FieldUtils>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, round: usize, num_bits: usize, input: Variable) -> Vec<Variable> {
    let bits = circuit.advice(
        round,
        Advice::new(1, num_bits, move |input: &[F], _|{
                let input = input[0];
                let limbs = input.to_repr();
                let mut ret = vec![];
                for limb in limbs.as_ref() {
                    for j in 0..8 {
                        if ret.len() < num_bits{
                            ret.push(F::from(((limb>>j)%2) as u64));
                        } else {
                            assert!((limb>>j) % 2 == 0, "An input {:?} is too large to be decomposed into {} bit", input.to_repr().as_ref(), num_bits);
                        }
                    }
                }
                ret
            }
        ),
        vec![input],
    );

    let bitcheck_gate = Gatebb::new(2, 1, 1, Rc::new(|args, _| bitcheck::<F>(args)), vec![]);

    for i in 0..num_bits-1 {
        circuit.constrain(&vec![bits[i]], bitcheck_gate.clone())
    }
    circuit.constrain(&vec![bits[num_bits-1]], bitcheck_gate);

    let decompcheck_gate = Gatebb::new(1, num_bits+1, 1, Rc::new(|args, _| decompcheck::<F>(args)), vec![]);
    let tmp : Vec<_> = repeat(input).take(1).chain(bits.iter().map(|x|*x)).collect();
    circuit.constrain(&tmp, decompcheck_gate);

    bits

}