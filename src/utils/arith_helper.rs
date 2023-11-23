use std::ops::ShrAssign;
use ff::PrimeField;
use num_traits::{Unsigned, PrimInt};

/// Returns k for numbers x such that 2^{k-1} < x <= 2^k. Panics for x=0.
pub fn log2_ceil<T: Unsigned + PrimInt + ShrAssign<i32>>(x: T) -> usize {
    assert!(x > T::zero(), "log2(0) = -infty");
    let mut x = x-T::one();
    let mut ret = 0;
    while x>T::zero() {
        ret +=1;
        x>>=1;
    }
    ret
}

pub fn ev<F:PrimeField>(poly: &Vec<F>, x: F) -> F {
    let mut ret = F::ZERO;
    let l = poly.len();
    for i in 0..l {
        ret *= x;
        ret += poly[l-i-1];
    }
    ret    
}

/// Converts jacobian to affine.
pub fn j2a<F: PrimeField>(pt: (F,F,F)) -> (F,F) {
    let zi = pt.2.invert().unwrap();
    let zisq = zi.square();
    let zicb = zisq*zi;
    (pt.0 * zisq, pt.1 * zicb)
}