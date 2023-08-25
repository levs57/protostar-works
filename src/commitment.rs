use group::Curve;
use halo2::arithmetic::best_multiexp;
use halo2curves::CurveAffine;

/// A simple commitment key.
pub enum CkS<G: CurveAffine>{
    Trivial,
    Group(Vec<G>),
}

/// Commitment target.
pub enum Ct<G: CurveAffine>{
    Trivial(Vec<G::Scalar>),
    Group(G),
}

/// Commitment key trait.
pub trait Ck<G: CurveAffine> {
    fn commit(&self, wtns: &Vec<G::Scalar>) -> Ct<G>;
} 

impl<G: CurveAffine> Ck<G> for CkS<G>{
    fn commit(&self, wtns: &Vec<G::Scalar>) ->  Ct<G>{
        match self {
            CkS::Trivial => Ct::Trivial(wtns.clone()),
            CkS::Group(v) => Ct::Group(best_multiexp(wtns, v).to_affine()),
        }
    }
}