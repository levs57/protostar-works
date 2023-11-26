// This module declares standard encoding of (potentially nonnative) field elements.

use std::iter::repeat;

use ff::PrimeField;
use itertools::Itertools;
use num_bigint::BigUint;
use num_traits::FromBytes;

use crate::utils::arith_helper::shift64;



pub trait Encoded<R: PrimeField> : PrimeField + Copy{
    fn encode(self) -> Vec<R> {
        let x = BigUint::from_le_bytes(self.to_repr().as_ref());
        let mut x = x.to_u64_digits();
        assert!(x.len() <= 4);
        x.extend(repeat(0).take(4-x.len()));
        let x = x.into_iter().map(|v|R::from(v)).collect_vec();
        vec![x[0]+shift64(x[1]), x[2]+shift64(x[3])]
    }
}

impl<R: PrimeField> Encoded<R> for R {
    fn encode(self) -> Vec<R> {
        vec![self]
    }
}