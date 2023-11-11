use ff::PrimeField;
use halo2::arithmetic::best_multiexp;
use halo2::halo2curves::CurveAffine;
use itertools::zip_eq;
use crate::witness::RoundWtns;

/// Commitment key trait.
pub trait CommitmentKey<G: CurveAffine> {
    type Scalars;
    type Target;
    
    fn commit(&self, wtns: &Self::Scalars) -> Self::Target;
} 

/// Witness commitment key for each round.
pub struct CkRound<G>(pub Vec<G>);

pub struct CtRound<F: PrimeField, G: CurveAffine<ScalarExt=F>>{
    pub pubs: Vec<F>,
    pub pt: G,
}

impl<F: PrimeField, G: CurveAffine<ScalarExt=F>> CommitmentKey<G> for CkRound<G>{
    type Scalars = RoundWtns<F>;
    type Target = CtRound<F, G>;

    fn commit(&self, wtns: &Self::Scalars) -> Self::Target {
        let pubs = wtns.pubs.iter().map(|x| x.expect("public input should be initialized")).collect();

        let scalars : Vec<_> = wtns.privs.iter().map(|x| x.expect("witness should be initialized")).collect();
        let pt = best_multiexp(&scalars, &self.0).into();

        Self::Target { pubs, pt }
    }
}

/// Commitment key for the full witness.
pub type CkWtns<G> = Vec<CkRound<G>>;
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

/// Commitment key for a relaxed instance
pub type CkRelaxed<G> = (CkWtns<G>, CkErr<G>);

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

// A vector of commitment keys is a commitment key for an appropriate vector of scalars
impl<F: PrimeField, G: CurveAffine<ScalarExt=F>, CK: CommitmentKey<G>> CommitmentKey<G> for Vec<CK> {
    type Scalars = Vec<CK::Scalars>;
    type Target = Vec<CK::Target>;

    fn commit(&self, wtns: &Self::Scalars) -> Self::Target {
        zip_eq(self, wtns).map(|(ck, wtns)| ck.commit(wtns)).collect()
    }
}

// A pair of commitment keys is a commitment key for an appropriate pair of scalars
impl<F: PrimeField, G: CurveAffine<ScalarExt = F>, CK1, CK2> CommitmentKey<G> for (CK1, CK2)
where
    CK1: CommitmentKey<G>,
    CK2: CommitmentKey<G>,
{
    type Scalars = (CK1::Scalars, CK2::Scalars);
    type Target = (CK1::Target, CK2::Target);

    fn commit(&self, wtns: &Self::Scalars) -> Self::Target {
        (self.0.commit(&wtns.0), self.1.commit(&wtns.1))
    }
}