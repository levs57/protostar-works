use std::iter::repeat;

use ff::PrimeField;
use group::Curve;
use halo2curves::CurveAffine;

use crate::{gate::{self, Gate, RootsOfUnity}, constraint_system::{self, ConstraintSystem, Variable, CommitKind}, commitment::{CommitmentKey, CtS, CkWtns, CkRound, CtRound, ErrGroup, CkErr, CkRelaxed}};

pub struct RoundWtns<F: PrimeField> {
    pub pubs: Vec<Option<F>>,
    pub privs: Vec<Option<F>>,
}

/// Trait which outputs full commitment (i.e. verifier view of an instance) from a fully populated commitment system.
pub trait CSSystemCommit<F: PrimeField, G: CurveAffine<ScalarExt=F>, CK: CommitmentKey<G>>{
    fn commit(&self, ck: &CK) -> CK::Target;
}


/// CS system + aux witness data.
pub struct CSWtns<'a, F: PrimeField> {
    pub cs : ConstraintSystem<'a, F>,
    pub wtns : Vec<RoundWtns<F>>,
}

impl<'a, F:PrimeField> CSWtns<'a, F>{

    pub fn new(cs: ConstraintSystem<'a, F>) -> Self{
        let mut wtns = vec![];
        for vg in &cs.vars {
            wtns.push(RoundWtns{pubs: repeat(None).take(vg.pubs).collect(), privs: repeat(None).take(vg.privs).collect()})
        }
        Self {cs, wtns}
    }

    // Should add error resolution at some point, for now it will just panic in case of double assignment.
    pub fn setvar(&mut self, var: Variable, value: F) -> (){
        match var {
            Variable::Public(r, i) => {
                match self.wtns[r].pubs[i] {None => (), _ => panic!("Double assignment error at public variable {}, {}.", r, i)};
                self.wtns[r].pubs[i] = Some(value)
            },
            Variable::Private(r, i) => {
                match self.wtns[r].privs[i] {None => (), _ => panic!("Double assignment error at private variable {}, {}.", r, i)};
                self.wtns[r].privs[i] = Some(value)
            },
        }
    }

    pub fn getvar(&self, var: Variable) -> F {
        match var {
            Variable::Public(r, i) => {
                match self.wtns[r].pubs[i] {Some(x)=>x, _=>panic!("Trying to retrieve unassigned public variable {}, {}.", r, i)}
            }
            Variable::Private(r, i) => {
                match self.wtns[r].privs[i] {Some(x)=>x, _=>panic!("Trying to retrieve unassigned private variable {}, {}.", r, i)}
            }
        }
    }

    pub fn alloc_pub_internal(&mut self, r: usize) -> Variable{
        self.wtns[r].pubs.push(None);
        self.cs.alloc_pub_internal(r)
    }

    pub fn alloc_priv_internal(&mut self, r: usize) -> Variable{
        self.wtns[r].privs.push(None);
        self.cs.alloc_priv_internal(r)
    }

    pub fn alloc_pub(&mut self) -> Variable {
        self.alloc_pub_internal(self.cs.num_rounds()-1)
    }

    pub fn alloc_priv(&mut self) -> Variable {
        self.alloc_priv_internal(self.cs.num_rounds()-1)
    }

    pub fn round_commit<G>(&self, round: usize, ck: CkRound<G>) -> CtRound<F, G> where G:CurveAffine<ScalarExt=F>{
        ck.commit(&self.wtns[round])
    }

    pub fn relax(self) -> CSWtnsRelaxed<'a, F> {
        let mut err = vec![];
        for cg in &self.cs.cs {
            err.push(
                match cg.kind {
                    CommitKind::Zero => ErrGroup::Zero,
                    CommitKind::Trivial => ErrGroup::Trivial(repeat(F::ZERO).take(cg.num_rhs).collect()),
                    CommitKind::Group => ErrGroup::Group(repeat(F::ZERO).take(cg.num_rhs).collect()),
                }
            )
        }
        CSWtnsRelaxed { cs: self, err }
    }

}

impl<'a, F: PrimeField, G:CurveAffine<ScalarExt=F>> CSSystemCommit<F, G, CkWtns<G>> for CSWtns<'a, F>{
    fn commit(&self, ck: &CkWtns<G>) -> Vec<CtRound<F, G>> {
        ck.commit(&self.wtns)
    }
}

pub struct CSWtnsRelaxed<'a, F: PrimeField> {
    cs: CSWtns<'a, F>,    
    err: Vec<ErrGroup<F>>
}

impl<'a, F: PrimeField, G:CurveAffine<ScalarExt=F>> CSSystemCommit<F, G, CkRelaxed<'a, G>> for CSWtnsRelaxed<'a, F>{
    fn commit(&self, ck: &CkRelaxed<G>) -> <CkRelaxed<G> as CommitmentKey<G>>::Target {
        ck.commit(&(&self.cs.wtns, &self.err))
    }
}
