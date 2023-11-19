// This module describes traits that we need for challenges, and implements some of those.

use std::marker::PhantomData;

use ff::{Field, PrimeField};
use rand_core::OsRng;

pub trait Oracle<ProverMsg, Response> {
    /// Initialize a new oracle.
    fn new() -> Self;
    fn update(&mut self, msg: ProverMsg);
    fn response(&self) -> Response;
}

pub struct MockOracle<R: PrimeField> {
    _marker: PhantomData<R>,
}

pub fn trunc128<R: PrimeField>(x:R) -> R {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&x.to_repr().as_ref()[0..8]);
    let low = R::from(u64::from_le_bytes(buf));
    buf.copy_from_slice(&x.to_repr().as_ref()[8..16]);
    let mut hi = R::from(u64::from_le_bytes(buf));
    for _ in 0..64 {
        hi=hi.double()
    }
    low+hi
}

impl<ProverMsg, R: PrimeField> Oracle<ProverMsg, R> for MockOracle<R> {
    fn new() -> Self {
        Self{ _marker: PhantomData::<R> }
    }
    fn update(&mut self, _msg: ProverMsg) {}
    fn response(&self) -> R {
        trunc128(R::random(OsRng))
    }
}



mod tests{
    use super::*;
    use halo2::halo2curves::bn256;
    

    type R = bn256::Fr;

    #[test]
    fn test_byte_decomposition() -> () {
        let x = R::random(OsRng);
        let tmp = x.to_repr();
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&tmp.as_ref()[0..8]);
        let a0 = R::from(u64::from_le_bytes(buf));
        buf.copy_from_slice(&tmp.as_ref()[8..16]);
        let mut a1 = R::from(u64::from_le_bytes(buf));
        for _ in 0..64 {
            a1=a1.double()
        }
        buf.copy_from_slice(&tmp.as_ref()[16..24]);
        let mut a2 = R::from(u64::from_le_bytes(buf));
        for _ in 0..2*64 {
            a2=a2.double()
        }
        buf = [0u8; 8];

        buf.copy_from_slice(&tmp.as_ref()[24..32]);
        let mut a3 = R::from(u64::from_le_bytes(buf));
        for _ in 0..3*64 {
            a3=a3.double()
        }
        
        assert_eq!(x, a0+a1+a2+a3)

    }
}