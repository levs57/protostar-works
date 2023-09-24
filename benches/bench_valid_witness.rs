use std::{rc::Rc, iter::repeat_with};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ff::Field;
use halo2curves::bn256;
use protostar_works::{gadgets::poseidon::{Poseidon, poseidon_gadget}, circuit::{ExternalValue, Circuit, Advice}, gate::{Gatebb, Gate}, constraint_system::Variable};
use rand_core::OsRng;


type F = bn256::Fr;

pub fn hundred_poseidons_witness_verification(c: &mut Criterion) {
    let pi = ExternalValue::new();
    let poseidon_consts = Poseidon::new();

    let mut circuit = Circuit::<F, Gatebb<F>>::new(25, 1);

    let load_pi_advice_head = Advice::new(0,1,1, Rc::new(|_, iext: &[F]| vec![iext[0]]));
    let mut acc = circuit.advice_pub(0, load_pi_advice_head, vec![], vec![&pi])[0];

    for _ in 0..100 {
        acc = poseidon_gadget(&mut circuit, &poseidon_consts, 1, 0, vec![acc]);
    }

    circuit.finalize();

    // FIXME: use criterion's randomness
    pi.set(F::random(OsRng)).unwrap();
    circuit.execute(0);

    c.bench_function("valid_witness 100 poseidons", |b| b.iter(|| circuit.cs.valid_witness()));
}

pub fn evaluate_on_random_linear_combinations(gate: impl Gate<F>, a: Vec<F>, b: Vec<F>) {
    // let a: Vec<_> = repeat_with(|| F::random(OsRng)).take(gate.i()).collect();
    // let b: Vec<_> = repeat_with(|| F::random(OsRng)).take(gate.i()).collect();
    let mut interpolation_values: Vec<Vec<F>> = Vec::with_capacity(gate.d());

    for i in 0..gate.d() {
        let t = F::random(OsRng);
        let fold: Vec<_> = a.iter().zip(b.iter()).map(|(&x, &y)| x + t * y).collect();

        let obuf = gate.exec(&fold);
        interpolation_values[i] = obuf;
    }

}

pub fn bench_

criterion_group!(benches, hundred_poseidons_witness_verification);
criterion_main!(benches);