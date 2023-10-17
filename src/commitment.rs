use ff::PrimeField;
use halo2::arithmetic::best_multiexp;
use halo2::halo2curves::CurveAffine;
use crate::witness::RoundWtns;

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
pub type CkRound<G> = Vec<G>;

pub struct CtRound<F: PrimeField, G: CurveAffine<ScalarExt=F>>{
    pub pubs: Vec<F>,
    pub pt: G,
}

impl<F: PrimeField, G: CurveAffine<ScalarExt=F>> CommitmentKey<G> for CkRound<G>{
    type Scalars = RoundWtns<F>;
    type Target = CtRound<F, G>;

    fn commit(&self, wtns: &Self::Scalars) -> Self::Target {
        let pubs = wtns.pubs.iter().map(|x|x.expect("public input should be initialized")).collect();
        let scalars : Vec<_> = wtns.privs.iter().map(|x|x.expect("witness should be initialized")).collect();
        let pt = best_multiexp(&scalars, &self).into();
        CtRound{pubs, pt}
    }
}

/// Commitment key for the full witness.
pub type CkWtns<G> = Vec<CkRound<G>>;

impl<F: PrimeField, G: CurveAffine<ScalarExt=F>> CommitmentKey<G> for CkWtns<G> {
    type Scalars = Vec<<CkRound<G> as CommitmentKey<G>>::Scalars>;
    type Target = Vec<<CkRound<G> as CommitmentKey<G>>::Target>;

    fn commit(&self, wtns: &Self::Scalars) -> Self::Target {
        self.iter().zip(wtns.iter()).map(|(ck,wtns)|ck.commit(wtns)).collect()
    }
}
pub enum ErrGroup<F: PrimeField>{
    Zero,
    Trivial(Vec<F>),
    Group(Vec<F>),
}
pub enum CkErrGroupTarget<F: PrimeField, G: CurveAffine<ScalarExt=F>>{
    Zero,
    Trivial(Vec<F>),
    Group(G),
}

pub type CkErrTarget<F, G> = Vec<CkErrGroupTarget<F,G>>;

/// Commitment key for the error term group.
pub enum CkErrGroup<G: CurveAffine>{
    Zero,
    Trivial,
    Group(Vec<G>),
}

/// Commitment key for the error term.
pub type CkErr<G> = Vec<CkErrGroup<G>>;

impl<F: PrimeField, G: CurveAffine<ScalarExt=F>> CommitmentKey<G> for CkErrGroup<G> {
    type Scalars = ErrGroup<F>;
    type Target = CkErrGroupTarget<F, G>;

    fn commit(&self, wtns: &Self::Scalars) -> Self::Target {
        match (self, wtns) {
            (Self::Zero, ErrGroup::Zero) => CkErrGroupTarget::Zero,
            (Self::Trivial, ErrGroup::Trivial(data)) => CkErrGroupTarget::Trivial(data.clone()),
            (Self::Group(ck), ErrGroup::Group(data)) => CkErrGroupTarget::Group(best_multiexp(&data, ck).into()),
            _ => panic!("Incompatible commitment key."),
        }
    }
} 

impl<F: PrimeField, G: CurveAffine<ScalarExt=F>> CommitmentKey<G> for CkErr<G> {
    type Scalars = Vec<<CkErrGroup<G> as CommitmentKey<G>>::Scalars>;
    type Target = Vec<<CkErrGroup<G> as CommitmentKey<G>>::Target>;

    fn commit(&self, wtns: &Self::Scalars) -> Self::Target {
        self.iter().zip(wtns.iter()).map(|(ck, wtns)|ck.commit(wtns)).collect()
    }
}

pub type CkRelaxed<G> = (CkWtns<G>, CkErr<G>);

impl<F: PrimeField, G: CurveAffine<ScalarExt=F>> CommitmentKey<G> for CkRelaxed<G> {
    type Scalars = (<CkWtns<G> as CommitmentKey<G>>::Scalars, <CkErr<G> as CommitmentKey<G>>::Scalars);
    type Target = (<CkWtns<G> as CommitmentKey<G>>::Target, <CkErr<G> as CommitmentKey<G>>::Target);

    fn commit(&self, wtns: &Self::Scalars) -> Self::Target {
        (self.0.commit(&wtns.0), self.1.commit(&wtns.1))
    }
} 