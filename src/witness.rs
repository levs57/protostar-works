use std::iter::repeat;

use ff::PrimeField;
use group::Curve;
use halo2curves::CurveAffine;

use crate::{gate::{self, Gate, RootsOfUnity}, constraint_system::{self, ConstraintSystem, Variable, CommitKind}, commitment::{CommitmentKey, CtS, CkWtns, CkRound, CtRound, ErrGroup, CkErr, CkRelaxed}};

#[derive(Clone)]
pub struct RoundWtns<F: PrimeField> {
    pub pubs: Vec<Option<F>>,
    pub privs: Vec<Option<F>>,
}

/// Trait which outputs full commitment (i.e. verifier view of an instance) from a fully populated commitment system.
pub trait CSSystemCommit<F: PrimeField, G: CurveAffine<ScalarExt=F>, CK: CommitmentKey<G>>{
    fn commit(&self, ck: &CK) -> CK::Target;
}


#[derive(Clone)]
/// CS system + aux witness data.
pub struct CSWtns<F: PrimeField, T: Gate<F>> {
    pub cs : ConstraintSystem<F, T>,
    pub wtns : Vec<RoundWtns<F>>,
}

impl<F:PrimeField, T: Gate<F>> CSWtns<F, T>{

    pub fn new(cs: ConstraintSystem<F, T>) -> Self{
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

    pub fn relax(self) -> CSWtnsRelaxed<F, T> {
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

    pub fn valid_witness(&self) -> () {
        for cg in &self.cs.cs {
            for constr in &cg.entries {
                let tmp : Vec<_> = constr.inputs.iter().map(|x|self.getvar(*x)).collect();
                constr.gate.exec(&tmp).iter().map(|ret| assert!(*ret == F::ZERO, "Some constraint is not satisfied")).count();
            }
        }
    }

}

impl<F: PrimeField, T: Gate<F>, G:CurveAffine<ScalarExt=F>> CSSystemCommit<F, G, CkWtns<G>> for CSWtns<F, T>{
    fn commit(&self, ck: &CkWtns<G>) -> Vec<CtRound<F, G>> {
        ck.commit(&self.wtns)
    }
}

pub struct CSWtnsRelaxed<F: PrimeField, T : Gate<F>> {
    cs: CSWtns<F, T>,    
    err: Vec<ErrGroup<F>>
}

impl<F: PrimeField, T: Gate<F>, G:CurveAffine<ScalarExt=F>> CSSystemCommit<F, G, CkRelaxed<G>> for CSWtnsRelaxed<F, T>{
    fn commit(&self, ck: &CkRelaxed<G>) -> <CkRelaxed<G> as CommitmentKey<G>>::Target {
        (ck.0.commit(&self.cs.wtns),  ck.1.commit(&self.err))
    }
}
