use std::{rc::Rc, iter::repeat_with};

use criterion::{criterion_group, criterion_main, Criterion};
use ff::Field;
use halo2::halo2curves::bn256;
use protostar_works::{gadgets::poseidon::{Poseidon, poseidon_gadget_internal}, circuit::{ExternalValue, Circuit, Advice, Build}, gate::{Gatebb, Gate}, utils::poly_utils::bits_le, commitment::CkRound, witness::CSSystemCommit};
use rand_core::OsRng;


type F = bn256::Fr;
type G = bn256::G1Affine;


pub fn homogenize<'a>(gate: Gatebb<'a, F>, mu: F) -> Gatebb<'a, F> {
    let mu_inv = mu.invert().expect("relaxation factor should not be zero");
    let mu_d = mu.pow(bits_le(gate.d()));

    let gate_d = gate.d();
    let gate_i = gate.i();
    let gate_o = gate.o();

    let f = move |input: &[F]| {
        let mut ibuf = Vec::with_capacity(input.len());
        for i in 0..input.len() {
            ibuf.push(input[i] * mu_inv);
        }

        let mut obuf = gate.exec(&ibuf);
        for i in 0..obuf.len() {
            obuf[i] *= mu_d;
        } 

        obuf
    };

    Gatebb::new(gate_d, gate_i, gate_o, Rc::new(f))
}

pub fn evaluate_on_random_linear_combinations(gate: &impl Gate<F>, a: &Vec<F>, b: &Vec<F>, randomness: &Vec<F>) {
    let mut interpolation_values: Vec<Vec<F>> = Vec::with_capacity(gate.d());

    for i in 0..gate.d() {
        let fold: Vec<_> = a.iter().zip(b.iter()).map(|(&x, &y)| x + randomness[i] * y).collect();

        let obuf = gate.exec(&fold);
        interpolation_values.push(obuf);
    }

}

pub fn assemble_poseidon_circuit<'a>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>, Build>, cfg: &'a Poseidon, pi: &'a ExternalValue<F>) {
    let load_pi_advice_head = Advice::new(0, 1, 1, |_, iext: &[F]| vec![iext[0]]);
    let mut acc = circuit.advice_pub(0, load_pi_advice_head, vec![], vec![pi])[0];

    for _ in 0..1000 {
        acc = poseidon_gadget_internal(circuit, cfg, 2, 0, vec![acc]);
    }
}

pub fn poseidons_pseudo_fold(c: &mut Criterion) {
    let pi = ExternalValue::new();
    let cfg = Poseidon::new();

    let mut circuit = Circuit::new(25, 1);
    assemble_poseidon_circuit(&mut circuit, &cfg, &pi);

    let mut circuit = circuit.finalize();

    pi.set(F::random(OsRng)).unwrap();
    circuit.execute(0);

    circuit.cs.valid_witness();
    println!("Total circuit size: private: {} public: {}", circuit.cs.wtns[0].privs.len(), circuit.cs.wtns[0].pubs.len());

    let mu = F::random(OsRng); // relaxation factor

    let mut bench_data = Vec::<(Gatebb<F>, Vec<F>, Vec<F>, Vec<F>)>::new();
    for constr in circuit.cs.cs.iter_constraints() {
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
    let pi = ExternalValue::new();
    let cfg = Poseidon::new();

    let mut circuit = Circuit::new(25, 1);
    assemble_poseidon_circuit(&mut circuit, &cfg, &pi);

    let mut circuit = circuit.finalize();

    pi.set(F::random(OsRng)).unwrap();
    circuit.execute(0);

    circuit.cs.valid_witness();

    let mut ck = Vec::with_capacity(circuit.cs.wtns.len());
    for rw in &circuit.cs.wtns {
        let rck: CkRound<G> = rw.privs.iter().map(|_| G::random(OsRng)).collect();
        ck.push(rck);
    }

    c.bench_function("poseidons msm", |b| b.iter(|| circuit.cs.commit(&ck)));
}

criterion_group!(poseidon, poseidons_pseudo_fold, poseidons_msm);
criterion_main!(poseidon);