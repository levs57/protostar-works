use std::collections::HashMap;

use ff::PrimeField;

use crate::constraint_system::{Variable, Constraint};
use crate::{witness::CSWtns, gate::Gate};
use crate::utils::discrete_ray::DiscreteRay;

pub struct RelaxedInstance<'c, F: PrimeField, G: Gate<'c, F>> {
    pub wtns: CSWtns<'c, F, G>,
    pub err_term: F,
}

// type RoundWitnessView<F> = (Vec<F>, Vec<F>);

// struct WitnessView<F> {
//     buffer: Vec<RoundWitnessView<F>>,
// }

// impl<F: Default + Clone> WitnessView<F> {
//     pub fn from_spec(spec: WitnessSpec) -> Self {
//         Self {
//             buffer: spec.into_iter().map(|RoundWitnessSpec(n_pubs, n_privs)| (vec![F::default(); n_pubs], vec![F::default(); n_privs])).collect(),
//         }
//     }
// }

pub fn constraint_multieval<'c, F: PrimeField, G: Gate<'c, F>>(constraint: &Constraint<'c, F, G>, line_evaluations: &HashMap<Variable, Vec<F>>) -> Vec<F> {
    (0..constraint.gate.d() + 1).map(|t| {
        let input: Vec<F> = constraint.inputs
            .iter()
            .map(|var| line_evaluations.get(var).expect("precomputed all variables")[t])
            .collect();

        constraint.gate.exec(&input, &constraint.constants)
    }).flatten().collect()
}

pub fn fold<'c, F: PrimeField, G: Gate<'c, F>>(a: RelaxedInstance<'c, F, G>, b: RelaxedInstance<'c, F, G>) {
    // WARNING: it is essential to check that we're folding the same circuits
    // TODO (aliventer): assert_eq!(instance_a.cs, instance_b.cs);

    // step 1: for each variable v, precompute A.v + t (B.v - A.v) for t = 0 .. max constraint degree of v (inclusive)
    let mut line_evaluations: HashMap<Variable, Vec<F>> = HashMap::new();
    for var in a.wtns.cs.iter_variables() {
        let av = a.wtns.getvar(var);
        let bv = b.wtns.getvar(var);
        let num_evals = a.wtns.cs.env[var].max_constraint_degree + 1;

        line_evaluations.insert(var, DiscreteRay::new(av, bv - av).take(num_evals).collect());
    }

    // step 2: evaluate each constraint f on A + t (B - A) for t = 0 .. deg f (inclusive)
    for constr in a.wtns.cs.iter_constraints() {
        let result = constraint_multieval(constr, &line_evaluations);
    }
}