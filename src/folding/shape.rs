// This file also implements folding utils like shape. Should split into parts later.


use ff::PrimeField;
use halo2::halo2curves::CurveAffine;
use itertools::Itertools;
use crate::{utils::arith_helper::{log2_ceil, ev}, constraint_system::{WitnessSpec, ProtoGalaxyConstraintSystem, CS, ConstrSpec}, gate::Gate, witness::Module};

/// Encode value as field elements.
pub trait FEncoding <F: PrimeField> {
    fn encode(&self) -> Vec<F>;
}

/// The shape of a circuit.
#[derive(Clone, Debug)]
pub struct Shape {
    pub wspec: WitnessSpec,
    pub cspec: ConstrSpec
}

impl Shape {
    pub fn new<'c,F:PrimeField,G:Gate<'c,F>>(c: &ProtoGalaxyConstraintSystem<'c, F, G>) -> Self {
        let wspec = c.witness_spec().clone();
        let cspec = c.constr_spec().clone();
        Self{wspec, cspec}
    }
}

#[derive(Clone, Debug)]
pub struct ProtostarLhs<F: PrimeField, C: CurveAffine<ScalarExt=F>> {
    pub round_commitments: Vec<C>,
    pub pubs: Vec<Vec<F>>,
    pub protostar_challenges: Vec<F>,
}

impl<F: PrimeField, C: CurveAffine<ScalarExt = F>> ProtostarLhs<F, C> {
    pub fn validate_shape(&self, shape: &Shape) {
        shape.wspec.round_specs.iter().zip_eq(self.pubs.iter())
            .map(|(rspec,rpubs)|{
                assert_eq!(rspec.pubs, rpubs.len())
            }).count();

        assert_eq!(self.pubs.len(), self.round_commitments.len());

        assert_eq!(self.protostar_challenges.len(), log2_ceil(shape.cspec.num_nonlinear_constraints));
    }
}

impl<F: PrimeField, C: CurveAffine<ScalarExt = F>> Module<F> for ProtostarLhs<F, C> {
    fn scale(&mut self, scale: F) {
        self.round_commitments.iter_mut().map(|x| *x = (*x * scale).into()).count();
        self.pubs.iter_mut().map(|rpubs| rpubs.iter_mut().map(|x| *x *= scale).count()).count();
        self.protostar_challenges.iter_mut().map(|x| *x *= scale).count();
    }

    fn neg(&mut self) {
        self.round_commitments.iter_mut().map(|x| *x = -*x).count();
        self.pubs.iter_mut().map(|rpubs| rpubs.iter_mut().map(|x| *x = -*x).count()).count();
        self.protostar_challenges.iter_mut().map(|x| *x = -*x).count();
    }

    fn add_assign(&mut self, other: Self) {
        self.round_commitments.iter_mut().zip_eq(other.round_commitments.iter())
            .map(|(a, b)| *a = (*a + *b).into()).count();
        self.pubs.iter_mut().zip_eq(other.pubs.iter())
            .map(|(rpubsa, rpubsb)| {
                rpubsa.iter_mut().zip_eq(rpubsb.iter())
                    .map(|(a, b)| *a += b).count()
            }).count();
        self.protostar_challenges.iter_mut().zip_eq(other.protostar_challenges.iter())
            .map(|(a, b)| *a += b).count();
    }
}

#[derive(Clone, Debug)]
pub struct ProtostarInstance<F: PrimeField, C: CurveAffine<ScalarExt = F>> {
    pub lhs: ProtostarLhs<F, C>,
    pub error: F,
}



pub struct Fold<F: PrimeField, C: CurveAffine<ScalarExt = F>> {
    acc: ProtostarInstance<F,C>,
    inc: ProtostarInstance<F,C>,
    // Represents a quotient of a line polynomial by t(t-1)
    // True line polynomial is thus represented as E0(1-t) + E1 t + t(t-1)v(t) 
    cross_terms: Vec<F>,
    challenge: Option<F>,
}

impl<F: PrimeField, C: CurveAffine<ScalarExt = F>> Fold<F,C> {
    pub fn new(acc: ProtostarInstance<F,C>, inc: ProtostarInstance<F,C>, cross_terms: Vec<F>, shape: Shape) -> Self {
        acc.lhs.validate_shape(&shape);
        inc.lhs.validate_shape(&shape);
        assert_eq!(cross_terms.len(), shape.cspec.max_degree + acc.lhs.protostar_challenges.len() - 1);
        Self { acc, inc, cross_terms, challenge: None }
    }

    pub fn challenge(&mut self, challenge:F) {
        assert!(self.challenge.is_none());
        self.challenge = Some(challenge);
    }

    pub fn fold(self) -> ProtostarInstance<F,C> {
        let Self{acc, inc, challenge, cross_terms} = self; 
        let ProtostarInstance{lhs: mut lhs_acc, error: error_acc} = acc;
        let ProtostarInstance{lhs: mut lhs_inc, error: error_inc} = inc;
        let t = challenge.unwrap();
        let mut tmp = lhs_acc.clone();
        tmp.neg();
        lhs_inc.add_assign(tmp);
        let mut diff = lhs_inc;
        diff.scale(t);
        lhs_acc.add_assign(diff);
        let lhs = lhs_acc;
        let nt = F::ONE-t;
        let error = nt*error_acc + t*error_inc + t*nt*ev(&cross_terms, t);
        ProtostarInstance { lhs, error }
    }
}