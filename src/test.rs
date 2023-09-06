use std::{rc::Rc, cell::{RefCell, Cell}};

use crate::{gate::{self, RootsOfUnity, Gatebb, Gate}, constraint_system::Variable, circuit::{Circuit, ExternalValue, PolyOp, Advice}};
use ff::{PrimeField, Field};
use halo2::arithmetic::best_fft;
use halo2curves::{bn256, serde::SerdeObject};
use num_traits::pow;
use rand_core::OsRng;

type F = bn256::Fr;

impl RootsOfUnity for F {
    /// Returns power of a primitive root of unity of order 2^logorder.
    fn roots_of_unity(power: u64, logorder: usize) -> Self{
        F::ROOT_OF_UNITY.pow([pow(2, F::S as usize - logorder)]).pow([power])
    }
    /// Returns power of 1/2.
    fn half_pow(power: u64) -> Self {
        F::TWO_INV.pow([power])
    }

    fn binomial_FFT(power: usize, logorder: usize) -> Vec<Self> {
        assert!(power < pow(2, logorder));
        let mut bin_coeffs = vec![1];
        for i in 1..pow(2,logorder) {
            let tmp = bin_coeffs[i-1];
            // n!/((i-1)!(n-i+1)!) * (n-i)/i
            if i <= power{
                bin_coeffs.push((tmp * (power-i+1)) / i)
            } else {
                bin_coeffs.push(0)
            }
        }
        let mut bin_coeffs : Vec<F>= bin_coeffs.iter().map(|x|F::from(*x as u64)).collect();
        let omega = F::roots_of_unity(1, logorder);
        best_fft(&mut bin_coeffs, omega, logorder as u32);
        bin_coeffs
    }
}

#[test]

fn test_cross_terms() {

    for d in 2..10{
        let f = Box::new(|v: &[F]| vec![v[0].pow([2 as u64])]);
        let gate = Gatebb::new(2, 1, 1, f);
        let tmp = gate.cross_terms_adjust(&vec![F::ONE], &vec![F::ONE], d);
        println!("{:?}", tmp.iter().map(|v|v[0]).collect::<Vec<_>>());
    }
}

#[test]

fn test_circuit_builder() {
    let public_input_source = ExternalValue::<F>::new();

    let mut circuit = Circuit::<F, Gatebb<F>>::new(2);

    let sq = PolyOp::new(2, 1, 1, Rc::new(|_: F, x|vec!(x[0]*x[0])));
    let input = circuit.advice_pub(Advice::new(0, 1, 1, Rc::new(|_, iext|vec![iext[0]])), vec![], vec![&public_input_source])[0];
    let sq1 = circuit.apply(sq.clone(), vec![input]);
    let output = circuit.apply_pub(sq.clone(), sq1);

    circuit.finalize();

    public_input_source.set(F::from(2)).unwrap();

    circuit.execute(0);

    println!("{:?}", circuit.cs.getvar(Variable::Public(0,2)).to_repr());
}

