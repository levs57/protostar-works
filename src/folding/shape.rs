// This file also implements folding utils like shape. Should split into parts later.

use ff::PrimeField;
use halo2::halo2curves::CurveAffine;
use itertools::Itertools;
use crate::utils::field_utils::FieldUtils;

/// Encode value as field elements.
pub trait FEncoding <F: PrimeField> {
    fn encode(&self) -> Vec<F>;
}

/// The shape of a circuit.
pub struct Shape {
    num_pubs: Vec<usize>,
    num_constraints: usize,
    max_degree: usize,
}

pub struct CommittedInstance<F: PrimeField, C: CurveAffine<ScalarExt=F>> {
    pubs: Vec<Vec<F>>,
    round_commitments: Vec<C>,
    num_constraints: usize,
    max_degree: usize,
}

impl<F: PrimeField + FieldUtils, C: CurveAffine<ScalarExt=F>> CommittedInstance<F, C> {
}

pub struct ProtostarCommitment<F: PrimeField, C: CurveAffine<ScalarExt=F>> {
    pubs: Vec<Vec<F>>,
    round_commitments: Vec<C>,
    protostar_challenges: Vec<F>,
    max_degree: usize,
}

impl<F: PrimeField + FieldUtils, C: CurveAffine<ScalarExt=F>> ProtostarCommitment<F,C> {
    pub fn of_shape(&self, shape: Shape) {
        self.pubs.iter().zip_eq(shape.num_pubs.iter()).map(|(x,y)|assert_eq!(x.len(), *y)).count();
    }

    pub fn fold(&self, other: Self) {

    }
}