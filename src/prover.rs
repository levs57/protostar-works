use ff::{PrimeField, BatchInvert};
use halo2::arithmetic::lagrange_interpolate;
use itertools::Itertools;

use crate::{witness::{ProtostarWtns, ProtostarLhsWtns}, gate::Gate, circuit::PolyOp, constraint_system::{ProtoGalaxyConstraintSystem, Visibility}, utils::{cross_terms_combination::{combine_cross_terms, self, EvalLayout}, field_precomp::FieldUtils, inv_lagrange_prod}, gadgets::range::lagrange_choice};

pub struct ProtoGalaxyProver {

}

impl ProtoGalaxyProver
{
    pub fn new() -> Self {
        Self {
            
        }
    }

    fn calculate_powers<
        'circuit, 
        F: PrimeField + FieldUtils, 
        G: Gate<'circuit, F> + From<PolyOp<'circuit, F>>
    > (
        &self,
        cs: &ProtoGalaxyConstraintSystem<'circuit, F, G>,
        template: &ProtostarLhsWtns<F>,
    ) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
        let mut pubs_degrees: Vec<Vec<usize>> = template.pubs.iter().map(|v| v.iter().map(|_| 0).collect_vec()).collect_vec();
        let mut privs_degrees: Vec<Vec<usize>> = template.round_wtns.iter().map(|v| v.iter().map(|_| 0).collect_vec()).collect_vec();

        for constraint in cs.iter_non_linear_constraints() {
            for variable in &constraint.inputs {
                match variable.visibility {
                    Visibility::Public => pubs_degrees[variable.round][variable.index] = pubs_degrees[variable.round][variable.index].max(constraint.gate.d()),
                    Visibility::Private => privs_degrees[variable.round][variable.index] = privs_degrees[variable.round][variable.index].max(constraint.gate.d()),
                }                
            }
        }
        (pubs_degrees, privs_degrees)
    }

    fn calculate_layout<
        'circuit, 
        F: PrimeField + FieldUtils, 
        G: Gate<'circuit, F> + From<PolyOp<'circuit, F>>
    > (
        &self,
        cs: &ProtoGalaxyConstraintSystem<'circuit, F, G>,
    ) -> Vec<EvalLayout> {
        let mut layout: Vec<EvalLayout> = vec![];
        for constraint in cs.iter_non_linear_constraints() {
            if layout.len() == 0 || layout[layout.len() - 1].deg != constraint.gate.d() {
                layout.push(EvalLayout{deg: constraint.gate.d(), amount: 0})
            }
            layout.last_mut().unwrap().amount += constraint.gate.o();
        }
        layout
    }

    fn build_variable_combinations_storage<F>(&self, degrees: &Vec<Vec<usize>>) -> Vec<Vec<Vec<F>>> {
        degrees.iter().map(|d| d.iter().map(|d| Vec::with_capacity(d + 1)).collect_vec()).collect_vec()
    }

    fn fill_variable_combinations<F: PrimeField + FieldUtils>(&self, mut storage: &mut Vec<Vec<Vec<F>>>, degrees: &Vec<Vec<usize>>, a: &Vec<Vec<F>>, b: &Vec<Vec<F>>) {
        storage.iter_mut().zip_eq(degrees.iter()).zip_eq(a.iter().zip_eq(b.iter())).map(|((s, d), (a, b))| {
            s.iter_mut().zip_eq(d.iter()).zip_eq(a.iter().zip_eq(b.iter())).map(|((res, d), (a, b))| {
                if *d != 0 { // min gate degree here is 2 (were iterating over nonlinear)
                    res.push(*a);
                    res.push(*b);
                    let diff = *b - *a;
                    let mut base = *b;
                    for _ in 1..*d {
                        base += diff;
                        res.push(base)
                    }
                }
            }).last()
        }).last();
    }

    fn combine_challenges<F: Clone>(&self, a: &Vec<F>, b: &Vec<F>) -> Vec<[F; 2]> {
        let pg_challenges = a.iter().zip_eq(b.iter()).map(|(a, b)| [a.clone(), b.clone()]).collect_vec();
        pg_challenges
    }

    fn prepare_interpolation_points<F: PrimeField>(&self, d: usize, log_ceil: usize) -> Vec<F> {
        (2..(d + log_ceil + 1)).map(|x| F::from(x as u64)).collect_vec()
    }

    fn leave_quotient<'a, F: PrimeField>(&self, evals: &'a mut Vec<F>) ->&'a [F]{
        let e_0 = evals[0];
        evals[0] = F::ZERO;
        let mut e_next = evals[1];
        evals[1] = F::ZERO;
        let diff = e_next - e_0;
        let mut invs = (2..evals.len()).map(|i| F::from((i * (i  - 1)) as u64)).collect_vec();
        invs.batch_invert();

        for (eval, inv) in evals.iter_mut().skip(2).zip_eq(invs.iter()) {
            e_next += diff;
            *eval -= e_next;
            *eval *= inv;
        }
        evals.split_at(2).1
    }

    fn evaluate<
        'circuit, 
        F: PrimeField + FieldUtils, 
        G: Gate<'circuit, F> + From<PolyOp<'circuit, F>>
    > (
        &self,
        cs: &ProtoGalaxyConstraintSystem<'circuit, F, G>,
        pubs_combinations: &Vec<Vec<Vec<F>>>,
        privs_combinations: &Vec<Vec<Vec<F>>>,
    ) -> Vec<F>{
        let mut evals = vec![];
        for constraint in cs.iter_non_linear_constraints() {
            for d in 0..constraint.gate.d() + 1 {
                evals.extend(constraint.gate.exec(&constraint.inputs.iter().map(|var| {
                    match var.visibility {
                        Visibility::Public => pubs_combinations[var.round][var.index][d],
                        Visibility::Private => privs_combinations[var.round][var.index][d],
                    }
                }).collect_vec()))
            }
        }
        evals
    }

    pub fn prove<'circuit, F: PrimeField + FieldUtils, G: Gate<'circuit, F> + From<PolyOp<'circuit, F>>>(
        &self,
        a: &ProtostarLhsWtns<F>, 
        b: &ProtostarLhsWtns<F>,
        cs: &ProtoGalaxyConstraintSystem<'circuit, F, G>,
    ) -> Vec<F> {
        let (pubs_degrees, privs_degrees) = self.calculate_powers(cs, a);
        let layout = self.calculate_layout(cs);

        let mut privs_combinations = self.build_variable_combinations_storage(&privs_degrees);
        let mut pubs_combinations = self.build_variable_combinations_storage(&pubs_degrees);

        // ^ that might be moved to 'new'

        self.fill_variable_combinations(&mut privs_combinations, &privs_degrees, &a.round_wtns, &b.round_wtns);
        self.fill_variable_combinations(&mut pubs_combinations, &pubs_degrees, &a.pubs, &b.pubs);

        let evals = self.evaluate(cs, &pubs_combinations, &privs_combinations);
        let pg_challenges = self.combine_challenges(&a.protostar_challenges, &b.protostar_challenges);
        let mut cross_terms = combine_cross_terms(evals, layout, pg_challenges);
        let cross_terms = self.leave_quotient(&mut cross_terms);
        
        let points = self.prepare_interpolation_points(cs.max_degree, a.protostar_challenges.len());
        lagrange_interpolate(&points, cross_terms)
    }
}

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use ff::Field;
    use halo2::halo2curves::bn256;
    use itertools::unfold;
    use rand_core::OsRng;
    use crate::{gate::Gatebb, circuit::{Circuit, Advice}, gadgets::input::input, witness::{Module, compute_error_term, ProtostarLhsWtns}};

    use super::*;

    #[test]
    fn pg_prover() {
        type F = bn256::Fr;
        let mut circuit = Circuit::new(4, 2);
        let inputs = circuit.ext_val(4);
        let input_vars = inputs.iter().map(|i| input(&mut circuit, *i, 0)).collect_vec();
        let mul_a_res = circuit.advice(1, Advice::new(
            2,
            1,
            |args, _| vec![args[0] * args[1]]
        ), vec![input_vars[0], input_vars[1]])[0];
        circuit.constrain(&[input_vars[0], input_vars[1], mul_a_res], Gatebb::<F>::new(2, 3, 1, Rc::new(|args, _| 
            {let res = vec![args[0] * args[1] - args[2]]; res}
        ), vec![]));

        let mul_b_res = circuit.advice(1, Advice::new(
            2,
            1,
            |args, _| vec![args[0] * args[1]]
        ), vec![input_vars[2], input_vars[3]])[0];
        circuit.constrain(&[input_vars[2], input_vars[3], mul_b_res], Gatebb::<F>::new(2, 3, 1, Rc::new(|args, _| vec![args[0] * args[1] - args[2]]), vec![]));
        
        let sum_res = circuit.advice(1, Advice::new(
            2,
            1,
            |args, _| vec![args[0] + args[1]]
        ), vec![mul_a_res, mul_b_res])[0];
        circuit.constrain(&[mul_a_res, mul_b_res, sum_res], Gatebb::<F>::new(1, 3, 1, Rc::new(|args, _| vec![args[0] + args[1] - args[2]]), vec![]));


        let constructed = circuit.finalize();

        let mut run_a = constructed.spawn();
        let mut run_b = constructed.spawn();

        for i in inputs {
            run_a.set_ext(i, F::random(OsRng));
            run_b.set_ext(i, F::random(OsRng));
        }
        
        run_a.execute(1);
        run_b.execute(1);

        let beta_a = F::random(OsRng);
        let beta_b = F::random(OsRng);

        let pgp = ProtoGalaxyProver::new();

        let a_wtns = run_a.end(beta_a);
        let b_wtns = run_b.end(beta_b);

        
        // Now we create random witnesses with same shape

        let a_wtns = ProtostarLhsWtns::random_like(&mut OsRng, &a_wtns.lhs);
        let b_wtns = ProtostarLhsWtns::random_like(&mut OsRng, &b_wtns.lhs);
        
        let q = pgp.prove(&a_wtns, &b_wtns, &constructed.circuit.cs);

        let a_err = compute_error_term(&a_wtns, &constructed.circuit.cs);
        let b_err = compute_error_term(&b_wtns, &constructed.circuit.cs);

        let t = F::random(OsRng);
        let mut fold_wtns = a_wtns.clone();
        fold_wtns.neg();
        fold_wtns.add_assign(b_wtns.clone());
        fold_wtns.scale(t);
        fold_wtns.add_assign(a_wtns.clone());

        let fold_err = compute_error_term(&fold_wtns, &constructed.circuit.cs);

        let q_eval: F = unfold(F::ONE, |next| {
            let tmp = *next;
            *next = *next * t;
            Some(tmp)
        }).zip(q).map(|(pow, c)| c * pow).sum();

        assert_eq!(fold_err, a_err + (b_err - a_err) * t + t * (t - F::ONE) * q_eval)
        
    }
}