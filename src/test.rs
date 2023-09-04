use crate::gate::{self, RootsOfUnity, Gatebb, Gate};
use ff::{PrimeField, Field};
use halo2::arithmetic::best_fft;
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

    fn binomial_FFT(power: usize, logorder: usize) -> Vec<Self> {
        assert!(power < pow(2, logorder));
        let mut bin_coeffs = vec![];
        bin_coeffs.push(1);
        for i in 1..logorder {
            let tmp = bin_coeffs[i-1];
            // n!/((i-1)!(n-i+1)!) * (n-i)/i
            if i <= power{
                bin_coeffs.push((tmp * (power-i)) / i)
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

    for d in 0..10{
        let f = Box::new(|v: &[F]| vec![v[0].pow([d as u64])]);
        let gate = Gatebb::new(d, 1, 1, f);
        let tmp = gate.cross_terms(&vec![F::ONE], &vec![F::ONE]);
        println!("{:?}", tmp.iter().map(|v|v[0]).collect::<Vec<_>>());
    }
}