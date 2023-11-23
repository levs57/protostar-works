// This implements a log-up lookup protocol.
// Few fixes that we need to eventually apply:
// 1. LOG-UP benefits greatly from the fact that a lot of values in it are zero.
// We are currently unable to exploit it.
// 2. Table right now is implemented as priveleged subset of variables. Considering it is the same for all
// step instances, it is not, actually, getting folded. This should be made a primitive.

use std::{iter::{once}, rc::Rc, collections::HashMap};

use ff::{PrimeField, BatchInvert};
use itertools::Itertools;
use num_bigint::BigUint;

use crate::{constraint_system::Variable, utils::field_precomp::FieldUtils,
    circuit::{Circuit, ExternalValue, Advice},
    gate::Gatebb,
    gadgets::{lc::{sum_gadget, inner_prod, sum_arr}, input::input, arith::eq_gadget}};

/// Outputs a product of vector elements and products skipping a single element.
pub fn montgomery<F: PrimeField+FieldUtils>(v: &[F]) -> (F, Vec<F>) {
    if v.len() == 0 {return (F::ONE, vec![])}

    let mut left = vec![F::ONE];
    let mut right = vec![F::ONE];

    let l = v.len();
    for i in 0..l-1 {
        let last = left[i];
        left.push(last*v[i]);
    }
    for i in 0..l-1 {
        let last = right[i];
        right.push(last*v[l-i-1]);
    }

    let prod = left[l-1]*v[l-1];

    let mut ret = vec![];

    for i in 0..l {
        ret.push(left[i]*right[l-i-1])
    }

    (prod, ret)
}

/// Parses input as `a, c, vals[0], ... vals[k-1]` and returns F::ZERO if a == \sum 1/(vals[i]-c) or one of denominators is F::ZERO itself
/// Assumes denominators are nonzero (which always happens provided c is a random challenge); unsound otherwise.
pub fn sum_of_fractions<F:PrimeField+FieldUtils> (args: &[F], k: usize) -> F {
    let (tmp, vals) = args.split_at(2);
    assert_eq!(vals.len(), k);
    let (res, c) = (tmp[0], tmp[1]);
    let (prod, skips) = montgomery(& vals.iter().map(|t| *t - c).collect_vec());
    res * prod - skips.iter().fold(F::ZERO, |acc, upd| acc + upd)
}

/// Parses input as `a, c, vals[0], ... vals[k-1], nums[0], ... nums[k-1]` and returns F::ZERO if a == \sum nums[i]/(vals[i]-c) or one of denominators is F::ZERO itself
pub fn sum_of_fractions_with_nums<F:PrimeField+FieldUtils> (args: &[F], dens: &[F], k: usize) -> F {
    let (tmp1, nums) = args.split_at(2);
    assert!(nums.len() == k);
    let (res, c) = (tmp1[0], tmp1[1]);
    let (prod, skips) = montgomery(& dens.iter().map(|t| *t - c).collect_vec());
    res * prod - skips.iter().zip_eq(nums.iter()).fold(F::ZERO, |acc, (skip, num)| acc + *skip * num)
}

/// Constrains res to be sum of inverses.
/// 
/// Exact form of the constraint is: res * \prod_i(vals[i] - challenge) - \sum_i(\prod_{j != i} vals[j]) == 0
/// 
/// # Panics
///
/// Panics if vals.len() == 0.
pub fn invsum_flat_constrain<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
    vals: &[Variable],
    res: Variable,
    challenge: Variable,
    ) -> (){
        assert!(vals.len() > 0);
        let args = [res, challenge].iter().chain(vals.iter()).map(|x| *x).collect_vec();
        let k = vals.len();
        let gate = Gatebb::new(vals.len() + 1, args.len(), 1, Rc::new(move |args, _|vec![sum_of_fractions(args, k)]), vec![]);
        circuit.constrain(&args, gate);        
    }

pub fn fracsum_flat_constrain<'a, 'c, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'c, F, Gatebb<'c, F>>,
    nums: &[Variable],
    dens: &'a [F],
    res: Variable,
    challenge: Variable,
) -> () {
    assert!(dens.len()==nums.len());
    let args = [res, challenge].iter().chain(nums.iter()).map(|x|*x).collect_vec();
    let k = dens.len();
    let gate = Gatebb::new(dens.len()+1, args.len(), 1, Rc::new(move |args, dens|vec![sum_of_fractions_with_nums(args, &dens, k)]), dens.to_vec());
    circuit.constrain(&args, gate);
}

/// Gadget which returns the sum of inverses of an array, shifted by a challenge.
/// Assumes that array length is divisible by rate.
/// Unsound if one of the inverses is undefined.
/// Rate - amount of values processed in a batch. Deg = rate+1
pub fn invsum_gadget<'a, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
    vals: &[Variable],
    challenge: Variable,
    rate: usize,
    round: usize,
    ) -> Variable {
        assert!(rate > 0);
        let l = vals.len();
        assert!(l%rate == 0);
        let mut vals = vals;
        let mut chunk;
        let advice = Advice::new(l+1, l/rate, move |args: &[F], _|{
            let (args, c) = args.split_at(l);
            let c = c[0];
            let mut inv = args.iter().map(|x|*x-c).collect_vec();
            inv.batch_invert();
            let mut ret = vec![];
            let mut inv : &[F] = &inv;
            let mut chunk;
            while inv.len() > 0 {
                (chunk, inv) = inv.split_at(rate);
                ret.push(sum_arr(chunk));
            }
            ret
        });

        let mut args = vals.to_vec();
        args.push(challenge);

        let batches = circuit.advice(round, advice, args);
        for i in 0..l/rate {
            (chunk, vals) = vals.split_at(rate);
            invsum_flat_constrain(circuit, chunk, batches[i], challenge);
        }

    sum_gadget(circuit, &batches, round)        

    }

/// Gadget which returns the sum of fractions of an array, shifted by a challenge.
/// Assumes that array length is divisible by rate, pad otherwise.
/// Unsound if one of the inverses is undefined.
/// Rate - amount of values processed in a batch. Deg = rate+1
pub fn fracsum_gadget<'a,'c, F: PrimeField+FieldUtils>(
    circuit: &mut Circuit<'c, F, Gatebb<'c, F>>,
    nums: &[Variable],
    dens: &'a [F],
    challenge: Variable,
    rate: usize,
    round: usize,
    ) -> Variable {
        assert!(rate > 0);
        assert!(nums.len() == dens.len());
        let l = nums.len();
        assert!(l%rate == 0);
        let mut nums = nums;
        let mut dens = dens;
        let captured_dens = dens.to_vec();
        let mut num_chunk;
        let mut den_chunk;
        let advice = Advice::<'c, F>::new(l+1, l/rate, move |args: &[F], _|{
            let (nums, c) = args.split_at(l);
            let c = c[0];
            let mut inv = captured_dens.iter().map(|x|*x-c).collect_vec();
            inv.batch_invert();
            let mut ret = vec![];
            let mut inv : &[F] = &inv;
            let mut nums : &[F] = &nums;
            let mut inv_chunk;
            let mut num_chunk;
            while inv.len() > 0 {
                (inv_chunk, inv) = inv.split_at(rate);
                (num_chunk, nums) = nums.split_at(rate);
                ret.push(inner_prod(inv_chunk, num_chunk));
            }
            ret
        });

        let args = nums.iter().map(|x|*x).chain(once(challenge)).collect();

        let batches = circuit.advice(round, advice, args);
        for i in 0..l/rate {
            (num_chunk, nums) = nums.split_at(rate);
            (den_chunk, dens) = dens.split_at(rate);
            fracsum_flat_constrain(circuit, num_chunk, den_chunk, batches[i], challenge);
        }

    sum_gadget(circuit, &batches, round)        
    }
/// 
pub trait Lookup<'a, F: PrimeField+FieldUtils> {
    /// Adds the variable to the list of variables to look up.
    fn check(&mut self, circuit: &mut Circuit<'a, F, Gatebb<'a,F>>, var: Variable) -> ();
    /// Seals the lookup and applies the constraints. Returns the challenge.
    /// Round parameter is the round of a challenge - so it must be strictly larger than rounds of any
    /// variable participating in a lookup.
    fn finalize(
        self,
        circuit: &mut Circuit<'a, F, Gatebb<'a,F>>,
        table_round: usize,
        access_round: usize,
        challenge_round: usize,
        rate: usize,
    ) -> ();
}

pub struct StaticLookup<F: PrimeField+FieldUtils> {
    vars: Vec<Variable>,
    round: usize,
    challenge: ExternalValue<F>,
    table: Vec<F>,
}

impl<F: PrimeField+FieldUtils> StaticLookup<F> {
    pub fn new<'c>(challenge_src: ExternalValue<F>, table: &'c [F]) -> Self {
        
        Self{
            vars: vec![],
            round: 0,
            challenge: challenge_src,
            table: table.to_vec(),
        }
    }
}

impl<'c, F: PrimeField+FieldUtils> Lookup<'c, F> for StaticLookup<F> {
    fn check(&mut self, _circuit: &mut Circuit<'c, F, Gatebb<'c,F>>, var: Variable) -> () {
        if self.round < var.round {
            self.round = var.round
        }
        self.vars.push(var);
    }
    fn finalize(
        self,
        circuit: &mut Circuit<'c, F, Gatebb<'c,F>>,
        table_round: usize,
        access_round: usize,
        challenge_round: usize,
        rate: usize,
    ) -> () {
        let Self{vars, round, challenge, table} = self;

        assert!(table_round <= access_round);
        assert!(access_round >= round);
        assert!(challenge_round > access_round);
        let mut table_hash = HashMap::new();
        table.iter().enumerate().map(|(i, var)| table_hash.insert(BigUint::from_bytes_le(var.to_repr().as_ref()), i)).last();
        // Access counts.
        let compute_accesses = Advice::new(vars.len(), table.len(), move |vars: &[F], _|{
            let mut ret = vec![0; table_hash.len()];
            for var in vars{
                let var = BigUint::from_bytes_le(var.to_repr().as_ref());
                let idx = match table_hash.get(&var) {
                    None => panic!("Error: lookup value {} out of range.", var),
                    Some(x) => *x,
                };
                ret[idx] += 1;
            }
            ret.into_iter().map(|x|F::from(x)).collect()
        });
        let access_counts = circuit.advice(access_round, compute_accesses, vars.clone());
        // Allocate challenge.
        let challenge = input(circuit, challenge, challenge_round);

        let lhs = invsum_gadget(circuit, &vars, challenge, rate, challenge_round);
        let rhs = fracsum_gadget(circuit, &access_counts, &table, challenge, rate, challenge_round);

        eq_gadget(circuit, lhs, rhs);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const TEST_LEN: usize = 12;
    use ff::Field;
    use halo2::halo2curves::bn256;
    use itertools::Itertools;
    use rand_core::OsRng;

    mod montgomery {
        use super::*;

        #[test]
        fn empty() {
            type F = bn256::Fr;
            assert_eq!(montgomery::<F>(&[]), (F::ONE, vec![]));
        }

        #[test]
        fn one() {
            type F = bn256::Fr;
            let points = [F::random(OsRng)]; 
            assert_eq!(montgomery::<F>(&points), (points[0], vec![F::ONE]));
        }

        #[test]
        fn random() {
            type F = bn256::Fr;
            let indexes = 0..TEST_LEN;
            let points = indexes.clone().map(|_| F::random(OsRng)).collect_vec();
            let (res_mul, res_partials) = montgomery::<F>(&points);
            assert_eq!(res_mul, points.iter().fold(F::ONE, |acc, n| acc * n));
            assert!(points.iter().zip_eq(res_partials).all(|(point, partial)| point * partial == res_mul))
        }
    }
    
    mod sum_of_fractions {
        use super::*;

        mod invalid {
            use super::*;

            #[test]
            #[should_panic]
            fn no_args() {
                type F = bn256::Fr;
                sum_of_fractions::<F>(&[], 0);
            }

            #[test]
            #[should_panic]
            fn small_k() {
                type F = bn256::Fr;
                sum_of_fractions::<F>(&[F::ONE, F::ONE, F::ONE, F::ONE], 1);
            }

            #[test]
            #[should_panic]
            fn big_k() {
                type F = bn256::Fr;
                sum_of_fractions::<F>(&[F::ONE, F::ONE, F::ONE, F::ONE], 3);
            }
        }


        #[test]
        fn empty() {
            type F = bn256::Fr;
            let c = F::random(OsRng);

            assert_eq!(sum_of_fractions::<F>(&[F::ZERO, c], 0), F::ZERO);
        }

        #[test]
        fn random_eq() {
            type F = bn256::Fr;
            let indexes = 0..TEST_LEN;
            let c = F::random(OsRng);
            let points = indexes.clone().map(|_| F::random(OsRng)).collect_vec();
            let sum = points.iter().map(|p| (p - c).invert().unwrap() * F::ONE).fold(F::ZERO, |acc, n| acc + n);
            let mut inputs = vec![sum, c];
            inputs.extend(points.iter());
            assert_eq!(sum_of_fractions::<F>(&inputs, indexes.len()), F::ZERO);
        }

        #[test]
        fn aware_constrain_random() {
            type F = bn256::Fr;
            let indexes = 0..TEST_LEN;
            let c = F::random(OsRng);
            let fake_sum = F::random(OsRng);
            let points = indexes.clone().map(|_| F::random(OsRng)).collect_vec();
            let mut inputs = vec![fake_sum, c];
            inputs.extend(points.iter());
            let res = sum_of_fractions::<F>(&inputs, indexes.len());
            let padded = points.iter().map(|p| (p - c)).collect_vec();
            let inverces = padded.iter().map(|p| p.invert().unwrap()).collect_vec();
            let real_sum = inverces.iter().fold(F::ZERO, |acc, n| acc + n);
            let real_denominator = padded.iter().fold(F::ONE, |acc, n| acc * n);
            let real_numerator = real_sum * real_denominator;
            assert_eq!(fake_sum * real_denominator - real_numerator, res);
        }
    }

    mod sum_of_fractions_with_nums {
        use super::*;

        mod invalid {
            use super::*;

            #[test]
            #[should_panic]
            fn no_args() {
                type F = bn256::Fr;
                sum_of_fractions_with_nums::<F>(&[], &[], 0);
            }

            #[test]
            #[should_panic]
            fn small_k() {
                type F = bn256::Fr;
                sum_of_fractions_with_nums::<F>(&[F::ONE, F::ONE], &[F::ONE, F::ONE, F::ONE], 1);
            }

            #[test]
            #[should_panic]
            fn big_k() {
                type F = bn256::Fr;
                sum_of_fractions_with_nums::<F>(&[F::ONE, F::ONE], &[F::ONE, F::ONE, F::ONE, F::ONE, F::ONE], 3);
            }
        }

        #[test]
        fn empty() {
            type F = bn256::Fr;
            let c = F::random(OsRng);

            assert_eq!(sum_of_fractions_with_nums::<F>(&[F::ZERO, c], &[], 0), F::ZERO);
        }

        #[test]
        fn random_eq() {
            type F = bn256::Fr;
            let indexes = 0..TEST_LEN;
            let c = F::random(OsRng);
            let points = indexes.clone().map(|_| F::random(OsRng)).collect_vec();
            let numerators = indexes.clone().map(|_| F::random(OsRng)).collect_vec();

            let sum = points.iter().zip_eq(&numerators).map(|(p, n)| (p - c).invert().unwrap() * n).fold(F::ZERO, |acc, n| acc + n);

            let mut inputs = vec![sum, c];
            inputs.extend(numerators.iter());
            assert_eq!(sum_of_fractions_with_nums::<F>(&inputs, &points, indexes.len()), F::ZERO);
        }

        #[test]
        fn aware_constrain_random() {
            type F = bn256::Fr;
            let indexes = 0..TEST_LEN;
            let c = F::random(OsRng);
            let fake_sum = F::random(OsRng);
            let points = indexes.clone().map(|_| F::random(OsRng)).collect_vec();
            let numerators = indexes.clone().map(|_| F::random(OsRng)).collect_vec();
            let mut inputs = vec![fake_sum, c];
            inputs.extend(numerators.iter());
            let res = sum_of_fractions_with_nums::<F>(&inputs, &points, indexes.len());
            let padded = points.iter().map(|p| (p - c)).collect_vec();
            let real_sum = padded.iter().zip_eq(&numerators).map(|(p, n)| p.invert().unwrap() * n).fold(F::ZERO, |acc, n| acc + n);
            let real_denominator = padded.iter().fold(F::ONE, |acc, n| acc * n);
            let real_numerator = real_sum * real_denominator;
            assert_eq!(fake_sum * real_denominator - real_numerator, res);
        }
    }

    mod invsum_flat_constrain {
        use super::*;

        #[test]
        fn random_eq() {
            type F = bn256::Fr;
            let indexes = 0..TEST_LEN;
            
            let challenge = F::random(OsRng);
            let points = indexes.clone().map(|_| F::random(OsRng)).collect_vec();
            let result = points.iter().map(|p| (p - challenge).invert().unwrap()).fold(F::ZERO, |acc, n| acc + n);

            let mut circuit = Circuit::new(TEST_LEN + 1, 1);
            let challenge_value = circuit.ext_val(1)[0];
            let points_values = circuit.ext_val(TEST_LEN);
            let result_value = circuit.ext_val(1)[0];

            let challenge_variable = input(&mut circuit, challenge_value, 0);
            let points_variables = points_values.clone().into_iter().map(|val| input(&mut circuit, val, 0)).collect_vec();
            let reslut_variable = input(&mut circuit, result_value, 0);

            invsum_flat_constrain(&mut circuit, &points_variables, reslut_variable, challenge_variable);
            
            let constructed = circuit.finalize();
            let mut instance = constructed.spawn();

            instance.set_ext(challenge_value, challenge);
            points_values.into_iter().zip_eq(points).map(|(val, point)| instance.set_ext(val, point)).last();
            instance.set_ext(result_value, result);

            instance.execute(0);
            instance.valid_witness();

        }
    }

    mod fracsum_flat_constrain {
        use super::*;

        #[test]
        fn random_eq() {
            type F = bn256::Fr;
            let indexes = 0..TEST_LEN;
            
            let challenge = F::random(OsRng);
            let points = indexes.clone().map(|_| F::random(OsRng)).collect_vec();
            let numerators = indexes.clone().map(|_| F::random(OsRng)).collect_vec();
            let result = points.iter().zip_eq(&numerators).map(|(p, n)| (p - challenge).invert().unwrap() * n).fold(F::ZERO, |acc, n| acc + n);


            let mut circuit = Circuit::new(TEST_LEN + 1, 1);
            let challenge_value = circuit.ext_val(1)[0];
            let numerators_values = circuit.ext_val(TEST_LEN);
            let result_value = circuit.ext_val(1)[0];

            let challenge_variable = input(&mut circuit, challenge_value, 0);
            let numerator_variables = numerators_values.clone().into_iter().map(|val| input(&mut circuit, val, 0)).collect_vec();
            let reslut_variable = input(&mut circuit, result_value, 0);

            fracsum_flat_constrain(&mut circuit, &numerator_variables, &points, reslut_variable, challenge_variable);
            
            let constructed = circuit.finalize();
            let mut instance = constructed.spawn();

            instance.set_ext(challenge_value, challenge);
            numerators_values.into_iter().zip_eq(numerators).map(|(val, point)| instance.set_ext(val, point)).last();
            instance.set_ext(result_value, result);

            instance.execute(0);
            instance.valid_witness();
        }
    }
    mod invsum_gadget{
        use super::*;

        #[test]
        fn random_eq() {
            type F = bn256::Fr;
            let indexes = 0..TEST_LEN;
            let rate = 3;
            let challenge = F::random(OsRng);
            let points = indexes.clone().map(|_| F::random(OsRng)).collect_vec();
            let result = points.iter().map(|p| (p - challenge).invert().unwrap()).fold(F::ZERO, |acc, n| acc + n);
            
            let mut circuit = Circuit::new(10, 1);
            let challenge_value = circuit.ext_val(1)[0];
            let points_values = circuit.ext_val(TEST_LEN);

            let challenge_variable = input(&mut circuit, challenge_value, 0);
            let points_variables = points_values.clone().into_iter().map(|val| input(&mut circuit, val, 0)).collect_vec();

            let result_variable = invsum_gadget(&mut circuit, &points_variables, challenge_variable, rate, 0);
            
            let constructed = circuit.finalize();
            let mut instance = constructed.spawn();

            instance.set_ext(challenge_value, challenge);
            points_values.into_iter().zip_eq(points).map(|(val, point)| instance.set_ext(val, point)).last();

            instance.execute(0);
            instance.valid_witness();

            assert_eq!(result, instance.cs.getvar(result_variable));
        }
    }

    mod fracsum_gadget {
        use super::*;

        #[test]
        fn random_eq() {
            type F = bn256::Fr;
            let indexes = 0..TEST_LEN;
            
            let challenge = F::random(OsRng);
            let points = indexes.clone().map(|_| F::random(OsRng)).collect_vec();
            let numerators = indexes.clone().map(|_| F::random(OsRng)).collect_vec();
            let result = points.iter().zip_eq(&numerators).map(|(p, n)| (p - challenge).invert().unwrap() * n).fold(F::ZERO, |acc, n| acc + n);

            let mut circuit = Circuit::new(TEST_LEN + 1, 1);
            let challenge_value = circuit.ext_val(1)[0];
            let numerators_values = circuit.ext_val(TEST_LEN);

            let challenge_variable = input(&mut circuit, challenge_value, 0);
            let numerator_variables = numerators_values.clone().into_iter().map(|val| input(&mut circuit, val, 0)).collect_vec();

            let result_variable = fracsum_gadget(&mut circuit, &numerator_variables, &points, challenge_variable, 3, 0);
            
            let constructed = circuit.finalize();
            let mut instance = constructed.spawn();

            instance.set_ext(challenge_value, challenge);
            numerators_values.into_iter().zip_eq(numerators).map(|(val, point)| instance.set_ext(val, point)).last();

            instance.execute(0);
            instance.valid_witness();
            assert_eq!(result, instance.cs.getvar(result_variable));
        }
    }

    mod range_lookup {
        use super::*;

        use rand_core::RngCore;

        #[test]
        fn random() {
            type F = bn256::Fr;
            let indices = 0..TEST_LEN;
            let range = 16;

            let table = (0..range).map(|_| F::random(OsRng)).collect_vec();
            let mut circuit = Circuit::new(range + 1, TEST_LEN + 1);

            let challenge_value = circuit.ext_val(1)[0];
            let test_values = circuit.ext_val(TEST_LEN);
            let mut range_lookup = StaticLookup::new(challenge_value, &table);

            let test_variables = test_values.clone().into_iter().enumerate().map(|(i, v)| input(&mut circuit, v, i)).collect_vec();
            test_variables.into_iter().map(|variable| range_lookup.check(&mut circuit, variable)).last();
            range_lookup.finalize(&mut circuit, 0, TEST_LEN - 1, TEST_LEN, 2);

            let constructed = circuit.finalize();
            let mut instance = constructed.spawn();

            test_values.into_iter().map(|val| instance.set_ext(val, table[(OsRng.next_u64() % range as u64) as usize])).last();
            for i in indices {
                instance.execute(i);
            }
            let challenge = F::random(OsRng);
            instance.set_ext(challenge_value, challenge);
            instance.execute(TEST_LEN);
            instance.valid_witness();
        }

        mod invalid {
            use super::*;

            #[test]
            #[should_panic]
            fn low_challendge_round() {
                type F = bn256::Fr;
                let range = 16;
                let rounds = 3;
    
                let table = (0..range).map(|x| F::from(x as u64)).collect_vec();
                let mut circuit = Circuit::new(range + 1, rounds);

                let challenge_value: ExternalValue<F> = circuit.ext_val(1)[0];
                let test_value = circuit.ext_val(1)[0];
                let mut range_lookup = StaticLookup::new(challenge_value, &table);

                let test_variable = input(&mut circuit, test_value, 0);
                range_lookup.check(&mut circuit, test_variable);
                range_lookup.finalize(&mut circuit, 1, 1, 1, 2);
            }

            #[test]
            #[should_panic]
            fn high_value_round() {
                type F = bn256::Fr;
                let range = 16;
                let rounds = 3;
    
                let table = (0..range).map(|x| F::from(x as u64)).collect_vec();
                let mut circuit = Circuit::new(range + 1, rounds);

                let challenge_value: ExternalValue<F> = circuit.ext_val(1)[0];
                let test_value = circuit.ext_val(1)[0];
                let mut range_lookup = StaticLookup::new(challenge_value, &table);

                let test_variable = input(&mut circuit, test_value, 2);
                range_lookup.check(&mut circuit, test_variable);
                range_lookup.finalize(&mut circuit, 1, 1, 1, 2);
            }

            #[test]
            #[should_panic]
            fn high_table_round() {
                type F = bn256::Fr;
                let range = 16;
                let rounds = 3;
    
                let mut circuit = Circuit::new(range + 1, rounds);
                
                let table = (0..range).map(|x| F::from(x as u64)).collect_vec();

                let challenge_value: ExternalValue<F> = circuit.ext_val(1)[0];
                let test_value = circuit.ext_val(1)[0];
                let mut range_lookup = StaticLookup::new(challenge_value, &table);

                let test_variable = input(&mut circuit, test_value, 2);
                range_lookup.check(&mut circuit, test_variable);
                range_lookup.finalize(&mut circuit, 1, 0, 2, 2);
            }
        }
    }
}