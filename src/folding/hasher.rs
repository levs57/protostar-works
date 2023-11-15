use ff::Field;
use halo2::halo2curves::bn256;

use super::poseidon::Poseidon;

pub trait Hasher<R: Field> {
    fn new() -> Self;
    fn hash(&self, input: Vec<R>) -> R;
}

impl Hasher<bn256::Fr> for Poseidon {
    fn new() -> Self {
        Poseidon::new()
    }
    fn hash(&self, input: Vec<bn256::Fr>) -> bn256::Fr {
        Poseidon::hash(&self, input)
    }
}