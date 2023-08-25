use crate::gate::{self, RootsOfUnity, Gatebb, Gate};
use ff::{PrimeField, Field};
use halo2curves::bn256;
use num_traits::pow;

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
}

#[test]

fn test_cross_terms() {

    for d in 0..10{
        let f = Box::new(|v: &Vec<F>| vec![v[0].pow([d as u64])]);
        let gate = Gatebb::new(d, 1, 1, f);
        let tmp = gate.cross_terms(&vec![F::ONE], &vec![F::ONE]);
        println!("{:?}", tmp.iter().map(|v|v[0]).collect::<Vec<_>>());
    }
}