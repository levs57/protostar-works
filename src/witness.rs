use std::iter::repeat;

use ff::PrimeField;
use halo2::halo2curves::CurveAffine;

use crate::{gate::Gate, constraint_system::{ConstraintSystem, Variable, CS, Visibility}, commitment::{CommitmentKey, CkWtns, CtRound, ErrGroup, CkRelaxed}};

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
pub struct CSWtns<F: PrimeField, G: Gate<F>> {
    pub cs : ConstraintSystem<F, G>,
    pub wtns : Vec<RoundWtns<F>>,
}

impl<F:PrimeField, G: Gate<F>> CSWtns<F, G>{

    pub fn new(cs: ConstraintSystem<F, G>) -> Self {
        let mut wtns = vec![];
        for round_spec in cs.witness_spec() {
            wtns.push(RoundWtns{pubs: vec![None; round_spec.0], privs: vec![None; round_spec.1]})
        }

        Self {cs, wtns}
    }

    pub fn setvar(&mut self, var: Variable, value: F) {
        let w = match var {
            Variable { visibility: Visibility::Public, round: r, index: i } => &mut self.wtns[r].pubs[i],
            Variable { visibility: Visibility::Private, round: r, index: i } => &mut self.wtns[r].privs[i],
        };

        assert!(w.is_none(), "Double assignment at variable {:?}", var);

        *w = Some(value);
    }

    // TODO: probably remove getvar & setvar, think of an api to get circuit's output variables (see this method references)
    pub fn getvar(&self, var: Variable) -> F {
        let w = match var {
            Variable { visibility: Visibility::Public, round: r, index: i } => self.wtns[r].pubs[i],
            Variable { visibility: Visibility::Private, round: r, index: i } => self.wtns[r].privs[i],
        };

        assert!(w.is_some(), "Use of unassigned variable: {:?}", var);

        w.expect("just asserted")
    }

    pub fn get_vars(&self, vars: &[Variable]) -> Vec<F> {
        vars.iter().map(|&v| self.getvar(v)).collect()
    }

    pub fn set_vars(&mut self, vars: &[(Variable, F)]) {
        for &(var, value) in vars {
            self.setvar(var, value);
        }
    }

    pub fn alloc_in_round(&mut self, round: usize, visibility: Visibility, size: usize) -> Vec<Variable> {
        let w = match visibility {
            Visibility::Public => &mut self.wtns[round].pubs,
            Visibility::Private => &mut self.wtns[round].privs,
        };

        w.extend(repeat(None).take(size));
        self.cs.alloc_in_round(round, visibility, size)
    }

    pub fn alloc(&mut self, visibility: Visibility, size: usize) -> Vec<Variable> {
        self.alloc_in_round(self.cs.last_round(), visibility, size)
    }

    // pub fn relax(self) -> CSWtnsRelaxed<F, G> {
    //     let mut err = vec![];
    //     for cg in &self.cs.cs {
    //         err.push(
    //             match cg.kind {
    //                 CommitKind::Zero => ErrGroup::Zero,
    //                 CommitKind::Trivial => ErrGroup::Trivial(repeat(F::ZERO).take(cg.num_rhs).collect()),
    //                 CommitKind::Group => ErrGroup::Group(repeat(F::ZERO).take(cg.num_rhs).collect()),
    //             }
    //         )
    //     }
    //     CSWtnsRelaxed { cs: self, err }
    // }

    pub fn valid_witness(&self) -> () {
        for constr in self.cs.iter_constraints() {
            let input_values: Vec<_> = constr.inputs.iter().map(|&x| self.getvar(x)).collect();
            let result = constr.gate.exec(&input_values);

            assert!(result.iter().all(|&output| output == F::ZERO), "Constraint {:?} is not satisfied", constr);
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
