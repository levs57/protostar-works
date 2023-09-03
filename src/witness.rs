use std::iter::repeat;

use ff::PrimeField;
use group::Curve;
use halo2curves::CurveAffine;

use crate::{gate::{self, Gate}, constraint_system::{self, ConstraintSystem, Variable, CommitKind}, commitment::{CommitmentKey, CtS, CkWtns, CkRound, CtRound, ErrGroup, CkErr, CkRelaxed}};

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
    pub cs : &'a ConstraintSystem<'a, F>,
    pub wtns : Vec<RoundWtns<F>>,
}

impl<'a, F:PrimeField> CSWtns<'a, F>{

    pub fn new(cs: &'a ConstraintSystem<'a, F>) -> Self{
        let mut wtns = vec![];
        for vg in &cs.vars {
            wtns.push(RoundWtns{pubs: repeat(None).take(vg.pubs).collect(), privs: repeat(None).take(vg.privs).collect()})
        }
        let mut prep = Self {cs, wtns};
        prep.setvar(Variable::Public(0,0), F::ONE);
        prep
    }

    // Should add error resolution at some point, for now it will just panic in case of double assignment.
    pub fn setvar(&mut self, var: Variable, value: F) -> (){
        match var {
            Variable::Public(r, i) => {
                match self.wtns[r].pubs[i] {None => (), _ => panic!("Double assignment error.")};
                self.wtns[r].pubs[i] = Some(value)
            },
            Variable::Private(r, i) => {
                match self.wtns[r].privs[i] {None => (), _ => panic!("Double assignment error.")};
                self.wtns[r].privs[i] = Some(value)
            },
        }
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


// Make CS system with partial witness / with full witness, so I can reuse it for NIFS.
// Do not choose how prover's advices works yet, just supply private inputs for each round.
// Gadget, by definition, takes partially computed witness, and computes a bit more data.
// It can depend on other arbitrary inputs.

// pub struct Circuit<'a, F : PrimeField, G : CurveAffine<Base=F>, CK: CommitmentKey<G, CtS<G>>> {
//     cs : ConstraintSystem<'a, F>,
//     pub_values : Vec<Option<F>>,
//     round_values : Vec<Vec<Option<F>>>,
//     challenges : Vec<Vec<Option<F>>>,
//     exec : Vec<Box<dyn 'a + FnMut(&mut Self) -> ()>>,
//     ck : CK,
// }



// pub fn apply<'a, F:PrimeField>(cs: &mut ConstraintSystem<'a, F>, gate: Box<dyn 'a + Gate<'a, F>>) {

// }