use ff::{Field, PrimeField};
use halo2::arithmetic::best_fft;
use halo2curves::bn256;
use num_traits::pow;

pub trait FieldUtils where Self : PrimeField{
    /// Returns power of a primitive root of unity of order 2^logorder.
    fn roots_of_unity(power: u64, logorder: usize) -> Self;
    /// Returns power of 1/2.
    fn half_pow(power: u64) -> Self;
    /// Returns FFT of the binomial.
    fn binomial_FFT(power: usize, logorder: usize) -> Vec<Self>;
    /// Multiplies the value by the small scalar.
    fn scale(&self, scale: u64) -> Self;
}

type F = bn256::Fr;

impl FieldUtils for F {
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

    /// Addition chains mostly taken from https://github.com/mratsim/constantine/blob/master/constantine/math/arithmetic/finite_fields.nim#L443 
    fn scale(&self, scale: u64) -> Self {
        let mut x = *self;
        let mut acc = Self::ZERO;
        if scale > 15 {
            let mut scale = scale;
            while scale > 0 {
                if scale%2 == 1 {
                    acc += x;
                }
                x = x.double();
                scale >>= 1;
            }
            acc
        } else {
            match scale {
                0 => F::ZERO,
                1 => x,
                2 => x.double(),
                3 => {let y = x.double(); y+x},
                4 => x.double().double(),
                5 => {let y = x.double().double(); y+x},
                6 => {x = x.double(); let y = x.double(); y+x},
                7 => {let y = x.double().double().double(); y-x},
                8 => {x.double().double().double()},
                9 => {let y = x.double().double().double(); y+x},
                10 => {x = x.double(); let y = x.double().double(); y+x},
                11 => {let y = x.double().double(); y.double()+y-x},
                12 => {let y = x.double().double(); y.double()+y},
                13 => {let y = x.double().double(); y.double()+y+x},
                14 => {x=x.double(); let y = x.double().double().double(); y-x},
                15 => {let y = x.double().double().double().double(); y-x},
                _ => unreachable!(),
            }
        }
    }

}