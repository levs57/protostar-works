use std::iter::repeat;
use std::marker::PhantomData;

use ff::PrimeField;
use halo2::halo2curves::CurveAffine;

use crate::{gate::Gate, constraint_system::{ConstraintSystem, Variable, CS, Visibility, WitnessSpec, RoundWitnessSpec}, commitment::{CommitmentKey, CkWtns, CtRound, ErrGroup, CkRelaxed}, circuit::ExternalValue};

#[derive(Clone)]
pub struct RoundWtns<F: PrimeField> {
    pub pubs: Vec<Option<F>>,
    pub privs: Vec<Option<F>>,
}

impl<F: PrimeField> RoundWtns<F> {
    pub fn with_spec(spec: RoundWitnessSpec) -> Self {
        let pubs = vec![None; spec.0];
        let privs = vec![None; spec.1];

        RoundWtns { pubs, privs }
    }
}

/// Trait which outputs full commitment (i.e. verifier view of an instance) from a fully populated commitment system.
pub trait CSSystemCommit<F: PrimeField, G: CurveAffine<ScalarExt=F>, CK: CommitmentKey<G>>{
    fn commit(&self, ck: &CK) -> CK::Target;
}


#[derive(Clone)]
/// Witness data.
pub struct CSWtns<'c, F: PrimeField, G: Gate<'c, F>> {
//    pub cs : ConstraintSystem<'c, F, G>,
    pub wtns : Vec<RoundWtns<F>>,
    pub ext_vals: Vec<Option<F>>,
    pub int_vals: Vec<Option<F>>,
    _marker: PhantomData<&'c G>,
}

impl<'c, F:PrimeField, G: Gate<'c, F>> CSWtns<'c, F, G>{

    pub fn new(cs: &ConstraintSystem<'c, F, G>) -> Self {
        
        let mut wtns = vec![];

        let WitnessSpec{round_specs, num_exts, num_ints} = cs.witness_spec();

        for round_spec in round_specs {
            wtns.push(RoundWtns{pubs: vec![None; round_spec.0], privs: vec![None; round_spec.1]})
        }

        let ext_vals = repeat(None).take(*num_exts).collect();
        let int_vals = repeat(None).take(*num_ints).collect();


        Self {wtns, ext_vals, int_vals, _marker: PhantomData::<&'c G>}
    }

    pub fn setvar(&mut self, var: Variable, value: F) {
        let w = match var {
            Variable { visibility: Visibility::Public, round: r, index: i } => &mut self.wtns[r].pubs[i],
            Variable { visibility: Visibility::Private, round: r, index: i } => &mut self.wtns[r].privs[i],
        };

        assert!(w.is_none(), "Double assignment at variable {:?}", var);

        *w = Some(value);
    }

    // TODO: think of an api to get circuit's output variables (see this method's references)
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

    pub fn getext(&self, ext: ExternalValue<F>) -> F {
        let e = self.ext_vals[ext.addr];
        assert!(e.is_some(), "Use of unassigned external value: {:?}", ext);
        e.unwrap()
    }

    pub fn setext(&mut self, ext: ExternalValue<F>, value: F) -> () {
        let e = &mut self.ext_vals[ext.addr];
        assert!(e.is_none(), "Double assignment at external value: {:?}", ext);
        *e = Some(value);
    }

    // pub fn alloc_in_round(&mut self, round: usize, visibility: Visibility, size: usize) -> Vec<Variable> {
    //     // let w = match visibility {
    //     //     Visibility::Public => &mut self.wtns[round].pubs,
    //     //     Visibility::Private => &mut self.wtns[round].privs,
    //     // };

    //     //w.extend(repeat(None).take(size));
    //     self.cs.alloc_in_round(round, visibility, size)
    // }

    // pub fn alloc(&mut self, visibility: Visibility, size: usize) -> Vec<Variable> {
    //     self.alloc_in_round(self.cs.last_round(), visibility, size)
    // }

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

    // pub fn valid_witness(&self) -> () {
    //     for constr in self.cs.iter_constraints() {
    //         assert!(constr.is_satisfied(self), "Constraint {:?} is not satisfied", constr);
    //     }
    // }

    pub fn witness_length(&self) -> usize {
        self.wtns.iter().map(|rw| rw.pubs.len() + rw.privs.len()).sum()
    }

}

impl<'c, F: PrimeField, T: Gate<'c, F>, G:CurveAffine<ScalarExt=F>> CSSystemCommit<F, G, CkWtns<G>> for CSWtns<'c, F, T>{
    fn commit(&self, ck: &CkWtns<G>) -> Vec<CtRound<F, G>> {
        ck.commit(&self.wtns)
    }
}

pub struct CSWtnsRelaxed<'c, F: PrimeField, T : Gate<'c, F>> {
    cs: CSWtns<'c, F, T>,    
    err: Vec<ErrGroup<F>>
}

impl<'c, F: PrimeField, T: Gate<'c, F>, G:CurveAffine<ScalarExt=F>> CSSystemCommit<F, G, CkRelaxed<G>> for CSWtnsRelaxed<'c, F, T>{
    fn commit(&self, ck: &CkRelaxed<G>) -> <CkRelaxed<G> as CommitmentKey<G>>::Target {
        (ck.0.commit(&self.cs.wtns),  ck.1.commit(&self.err))
    }
}
