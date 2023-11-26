use std::iter::repeat;
use std::marker::PhantomData;

use ff::PrimeField;
use halo2::{halo2curves::CurveAffine, arithmetic::best_multiexp};
use itertools::Itertools;
use rand_core::RngCore;

use crate::{gate::Gate, constraint_system::{ProtoGalaxyConstraintSystem, Variable, CS, Visibility, WitnessSpec}, commitment::{CommitmentKey, CkWtns, CtRound, ErrGroup, CkRelaxed}, circuit::{ExternalValue, ConstructedCircuit, PolyOp}, utils::field_precomp::FieldUtils, folding::shape::{ProtostarLhs, ProtostarInstance}};

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
/// Witness data.
pub struct CSWtns<'c, F: PrimeField, G: Gate<'c, F>> {
//    pub cs : ConstraintSystem<'c, F, G>,
    pub wtns : Vec<RoundWtns<F>>,
    pub ext_vals: Vec<Option<F>>,
    pub int_vals: Vec<Option<F>>,
    _marker: PhantomData<&'c G>,
}

impl<'c, F:PrimeField, G: Gate<'c, F>> CSWtns<'c, F, G>{

    pub fn new(cs: &ProtoGalaxyConstraintSystem<'c, F, G>) -> Self {
        
        let mut wtns = vec![];

        let WitnessSpec{round_specs, num_exts, num_ints} = cs.witness_spec();

        for round_spec in round_specs {
            wtns.push(RoundWtns{pubs: vec![None; round_spec.pubs], privs: vec![None; round_spec.privs]})
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

pub trait Module<F> {
    fn add_assign(&mut self, other: Self) -> ();
    fn neg(&mut self) -> ();
    fn scale(&mut self, scale: F) -> ();
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProtostarLhsWtns<F: PrimeField> {
    pub round_wtns: Vec<Vec<F>>,
    pub pubs: Vec<Vec<F>>,
    pub protostar_challenges: Vec<F>,
}

impl<F: PrimeField> ProtostarLhsWtns<F> {
    pub fn commit<C: CurveAffine<ScalarExt=F>> (&self, commitment_key: &Vec<Vec<C>>) -> ProtostarLhs<F, C> {
        ProtostarLhs { 
            round_commitments: self.round_wtns.iter().zip_eq(commitment_key).map(|(wtns, ck)| best_multiexp(&wtns, &ck).into()).collect_vec(),
            pubs: self.pubs.clone(),
            protostar_challenges: self.protostar_challenges.clone(),
        }
    }

    pub fn random_like<RNG: RngCore>(mut rng: &mut RNG, other: &Self) -> Self {
        Self { 
            round_wtns: other.round_wtns.iter().map(|r| r.iter().map(|_| F::random(&mut rng)).collect_vec()).collect_vec(),
            pubs: other.pubs.iter().map(|r| r.iter().map(|_| F::random(&mut rng)).collect_vec()).collect_vec(),
            protostar_challenges: other.protostar_challenges.iter().map(|_| F::random(&mut rng)).collect_vec(),
        }
    }
}

impl<F: PrimeField> Module<F> for ProtostarLhsWtns<F> {
    fn add_assign(&mut self, other: Self) -> () {
        self.round_wtns.iter_mut().zip_eq(other.round_wtns.iter()).for_each(|(s, o)| {
            s.iter_mut().zip_eq(o.iter()).for_each(|(s, o)| *s = *s + o)
        });
        self.pubs.iter_mut().zip_eq(other.pubs.iter()).for_each(|(s, o)| {
            s.iter_mut().zip_eq(o.iter()).for_each(|(s, o)| *s = *s + o)
        });
        self.protostar_challenges.iter_mut().zip_eq(other.protostar_challenges.iter()).for_each(|(s, o)| {
            *s = *s + o
        });
    }

    fn neg(&mut self) -> () {
        self.round_wtns.iter_mut().for_each(|s| {
            s.iter_mut().for_each(|s| *s = -*s)
        });
        self.pubs.iter_mut().for_each(|s| {
            s.iter_mut().for_each(|s| *s = -*s)
        });
        self.protostar_challenges.iter_mut().for_each(|s| {
            *s = -*s
        });
    }

    fn scale(&mut self, scale: F) -> () {
        self.round_wtns.iter_mut().for_each(|s| {
            s.iter_mut().for_each(|s| *s = *s * scale)
        });
        self.pubs.iter_mut().for_each(|s| {
            s.iter_mut().for_each(|s| *s = *s * scale)
        });
        self.protostar_challenges.iter_mut().for_each(|s| {
            *s = *s * scale
        });
    }
}


#[derive(Clone, PartialEq, Eq)]
pub struct ProtostarWtns<F: PrimeField> {
    pub lhs: ProtostarLhsWtns<F>,
    pub error: F
}

impl<F: PrimeField> Module<F> for ProtostarWtns<F> {
    fn add_assign(&mut self, other: Self) -> () {
        self.error += other.error;
        self.lhs.add_assign(other.lhs);
    }

    fn neg(&mut self) -> () {
        self.error = -self.error;
        self.lhs.neg();
    }

    fn scale(&mut self, scale: F) -> () {
        self.error *= scale;
        self.lhs.scale(scale);
    }
}

impl<F: PrimeField> ProtostarWtns<F> {
    pub fn commit<C: CurveAffine<ScalarExt=F>> (&self, commitment_key: &Vec<Vec<C>>) -> ProtostarInstance<F, C> {
        ProtostarInstance {
            lhs: self.lhs.commit(commitment_key),
            error: self.error,
        }
    }

    pub fn random_like<RNG: RngCore>(mut rng: &mut RNG, other: &Self) -> Self {
        Self {
            lhs: ProtostarLhsWtns::random_like(rng, &other.lhs),
            error: other.error,
        }
    }
}

pub fn compute_error_term<'circuit, F: PrimeField, G: Gate<'circuit, F>>(wtns: &ProtostarLhsWtns<F>, cs: &ProtoGalaxyConstraintSystem<'circuit, F, G>) -> F {
    let betas = &wtns.protostar_challenges;
    let mut results = vec![];
    for constr in cs.iter_non_linear_constraints() {
        let input_values: Vec<_> = constr.inputs.iter().map(|&x| match x.visibility {
            Visibility::Public => wtns.pubs[x.round][x.index],
            Visibility::Private => wtns.round_wtns[x.round][x.index],
        }).collect();
        results.extend(constr.gate.exec(&input_values));
    }

    assert!(betas.len() > 0, "No challenges supplied for error_term");
    let mut mid = 1 << (betas.len() - 1);
    assert!(mid < results.len());
    assert!(mid * 2 >= results.len());
    
    // example
    // results      : |.|.|.|.|.|.|.|
    // idx          :  0 1 2 3 4 5 6 
    // mid          :          ^
    // beta^4       : | | | | |1|1|1|
    // beta^2       : | | |1|1| | |1|
    // beta^1       : | |1| |1| |1| |
    //
    // split at mid : |.|.|.|.|   |.|.|.|
    // old idx      :  0 1 2 3     4 5 6 
    //
    // Now we multiply right half by beta^4 and element-wise add it to first half inplace
    //
    // results      : | r0 + r4 * beta^4 | r1 + r5 * beta^4 | r2 + r6 * beta^4 | r3 | junk | junk | junk |
    //
    // Now we forget about junk und repeat with first half until single element is left.

    for beta_pow in betas.iter().rev() {
        let (left, right) = results.split_at_mut(mid);
        for (l, r) in left.iter_mut().zip(right.iter()) {
            *l = *l + *r * beta_pow;
        }
        mid /= 2;
    }
    
    results[0]
}
