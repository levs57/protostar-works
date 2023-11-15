use std::{rc::Rc, iter::repeat_with};

use criterion::{criterion_group, criterion_main, Criterion};
use ff::Field;
use halo2::halo2curves::bn256;
use protostar_works::{gadgets::{poseidon::{poseidon_gadget_internal}, input::input}, circuit::{ExternalValue, Circuit, Advice}, gate::{Gatebb, Gate}, utils::poly_utils::bits_le, commitment::CkRound, witness::CSSystemCommit, folding::poseidon::Poseidon};
use rand_core::OsRng;


type F = bn256::Fr;
type G = bn256::G1Affine;


pub fn homogenize<'a>(gate: Gatebb<'a, F>, mu: F) -> Gatebb<'a, F> {
    let mu_inv = mu.invert().expect("relaxation factor should not be zero");
    let mu_d = mu.pow(bits_le(gate.d()));

    let gate_d = gate.d();
    let gate_i = gate.i();
    let gate_o = gate.o();

    let f = move |input: &[F], _: &[F]| {
        let mut ibuf = Vec::with_capacity(input.len());
        for i in 0..input.len() {
            ibuf.push(input[i] * mu_inv);
        }

        let mut obuf = gate.exec(&ibuf, &[]);
        for i in 0..obuf.len() {
            obuf[i] *= mu_d;
        } 

        obuf
    };

    Gatebb::new(gate_d, gate_i, gate_o, Rc::new(f))
}

pub fn evaluate_on_random_linear_combinations<'c>(gate: &impl Gate<'c, F>, a: &Vec<F>, b: &Vec<F>, randomness: &Vec<F>) {
    let mut interpolation_values: Vec<Vec<F>> = Vec::with_capacity(gate.d());

    for i in 0..gate.d() {
        let fold: Vec<_> = a.iter().zip(b.iter()).map(|(&x, &y)| x + randomness[i] * y).collect();

        let obuf = gate.exec(&fold, &[]);
        interpolation_values.push(obuf);
    }

}

pub fn assemble_poseidon_circuit<'a>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, cfg: &'a Poseidon, pi: ExternalValue<F>) {
    let mut acc = input(circuit, pi, 0);

    for _ in 0..1000 {
        acc = poseidon_gadget_internal(circuit, cfg, 2, 0, vec![acc]);
    }
}

pub fn poseidons_pseudo_fold(c: &mut Criterion) {
    let cfg = Poseidon::new();

    let mut circuit = Circuit::new(25, 1);
    let pi = circuit.ext_val(1)[0];

    assemble_poseidon_circuit(&mut circuit, &cfg, pi);

    let mut instance = circuit.finalize();

    instance.set_ext(pi, F::random(OsRng));
    instance.execute(0);

    instance.valid_witness();
    println!("Total circuit size: private: {} public: {}", instance.cs.wtns[0].privs.len(), instance.cs.wtns[0].pubs.len());

    let mu = F::random(OsRng); // relaxation factor

    let mut bench_data = Vec::<(Gatebb<F>, Vec<F>, Vec<F>, Vec<F>)>::new();
    for constr in instance.iter_constraints() {
        let gate = homogenize(constr.gate.clone(), mu);

        let a: Vec<_> = repeat_with(|| F::random(OsRng)).take(gate.i()).collect();
        let b: Vec<_> = repeat_with(|| F::random(OsRng)).take(gate.i()).collect();

        let randomness: Vec<_> = repeat_with(|| F::random(OsRng)).take(gate.d()).collect();

        bench_data.push((gate, a, b, randomness));
    }

    c.bench_function("poseidons pseudo fold", |b| b.iter(|| {
        bench_data.iter().for_each(|(gate, a, b, randomness)| evaluate_on_random_linear_combinations(gate, a, b, randomness))
    }));

}

pub fn poseidons_msm(c: &mut Criterion) {
    let cfg = Poseidon::new();

    let mut circuit = Circuit::new(25, 1);
    let pi = circuit.ext_val(1)[0];
    assemble_poseidon_circuit(&mut circuit, &cfg, pi);

    let mut instance = circuit.finalize();

    instance.set_ext(pi, F::random(OsRng));
    instance.execute(0);

    instance.valid_witness();

    let mut ck = Vec::with_capacity(instance.cs.wtns.len());
    for rw in &instance.cs.wtns {
        let rck: CkRound<G> = rw.privs.iter().map(|_| G::random(OsRng)).collect();
        ck.push(rck);
    }

    c.bench_function("poseidons msm", |b| b.iter(|| instance.cs.commit(&ck)));
}

criterion_group!(poseidon, poseidons_pseudo_fold, poseidons_msm);
criterion_main!(poseidon);