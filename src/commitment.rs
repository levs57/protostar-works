use ff::PrimeField;
use group::Curve;
use halo2::arithmetic::best_multiexp;
use halo2curves::CurveAffine;
use crate::{gadget::RoundWtns, constraint_system::CommitKind};

/// A simple commitment key.
pub enum CkS<G: CurveAffine>{
    Zero,
    Trivial,
    Group(Vec<G>),
}

/// Commitment target.
pub enum CtS<G: CurveAffine>{
    Zero,
    Trivial(Vec<G::Scalar>),
    Group(G),
}

/// Properties of the commitment target
pub trait CommitmentTarget{
}

impl<G: CurveAffine> CommitmentTarget for CtS<G>{
}

/// Commitment key trait.
pub trait CommitmentKey<G: CurveAffine> {
    type Scalars;
    type Target;
    fn commit(&self, wtns: &Self::Scalars) -> Self::Target;
} 

/// Witness commitment key for each round.
pub struct CkRound<G: CurveAffine>{
    pub key: Vec<G>,
}

pub struct CtRound<F: PrimeField, G: CurveAffine<Scalar=F>>{
    pubs: Vec<F>,
    pt: G,
}

impl<F: PrimeField, G: CurveAffine<Scalar=F>> CommitmentKey<G> for CkRound<G>{
    type Scalars = RoundWtns<G>;
    type Target = CtRound<F, G>;

    fn commit(&self, wtns: &Self::Scalars) -> Self::Target {
        let pubs = wtns.pubs.iter().map(|x|x.unwrap()).collect();
        let pt = best_multiexp(wtns.privs.iter().map(|x|x.unwrap()).collect(), &self.key);
        CtRound{pubs, pt}
    }
}

/// Commitment key for the full witness.
pub type CkWtns<G: CurveAffine> = Vec<CkRound<G>>;

impl<F: PrimeField, G: CurveAffine<Scalar=F>> CommitmentKey<G> for CkWtns<G> {
    type Scalars = Vec<RoundWtns<G>>;
    type Target = Vec<CtRound<F,G>>;

    fn commit(&self, wtns: &Self::Scalars) -> Self::Target {
        self.iter().zip(wtns.iter()).map(|(ck,wtns)|ck.commit(wtns)).collect()
    }
}

pub enum CkErrGroupTarget<F: PrimeField, G: CurveAffine<Scalar=F>>{
    Zero,
    Trivial(Vec<F>),
    Group(G),
}

pub type CkErrTarget<F: PrimeField, G: CurveAffine<Scalar=F>> = Vec<CkErrGroupTarget<F,G>>;

/// Commitment key for the error term group.
pub enum CkErrGroup<G: CurveAffine>{
    Zero,
    Trivial,
    Group(Vec<G>),
}

/// Commitment key for the error term.
pub type CkErr<G: CurveAffine> = Vec<CkErrGroup<G>>;

impl<F: PrimeField, G: CurveAffine<Scalar=F>> CommitmentKey<G> for CkErrGroup<G> {
    type Scalars = (Vec<F>, CommitKind);
    type Target = CkErrGroupTarget<F, G>;

    fn commit(&self, wtns: &Self::Scalars) -> Self::Target {
        match (self, wtns.1) {
            (Self::Zero, CommitKind::Zero) => CkErrGroupTarget::Zero,
            (Self::Trivial, CommitKind::Trivial) => CkErrGroupTarget::Trivial(wtns.0),
            (Self::Group(ck), CommitKind::Group) => best_multiexp(&wtns.0, ck),
            _ => panic!("Incompatible commitment key."),
        }
    }
} 

impl<F: PrimeField, G: CurveAffine<Scalar=F>> CommitmentKey<G> for CkErr<G> {
    type Scalars = Vec<<CkErrGroup<G> as CommitmentKey<G>>::Scalars>;
    type Target = Vec<<CkErrGroup<G> as CommitmentKey<G>>::Target>;

    fn commit(&self, wtns: &Self::Scalars) -> Self::Target {
        self.iter().zip(wtns.iter()).map(|(ck, wtns)|ck.commit(wtns)).collect()
    }
}