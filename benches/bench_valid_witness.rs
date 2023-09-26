use std::{rc::Rc, iter::repeat_with};

use criterion::{criterion_group, criterion_main, Criterion};
use ff::Field;
use halo2::circuit;
use halo2curves::bn256;
use protostar_works::{gadgets::poseidon::{Poseidon, poseidon_gadget}, circuit::{ExternalValue, Circuit, Advice}, gate::{Gatebb, Gate}, utils::poly_utils::bits_le, commitment::{CkRound, CommitmentKey}, witness::CSSystemCommit};
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

    // let f = move |input: &[F]| {
    //     gate.exec(input)
    // };

    Gatebb::new(gate_d, gate_i, gate_o, Rc::new(f))
}

pub fn evaluate_on_random_linear_combinations(gate: &impl Gate<F>, a: &Vec<F>, b: &Vec<F>) {
    let mut interpolation_values: Vec<Vec<F>> = Vec::with_capacity(gate.d());

    for _ in 0..gate.d() {
        let t = F::random(OsRng);
        let fold: Vec<_> = a.iter().zip(b.iter()).map(|(&x, &y)| x + t * y).collect();

        let obuf = gate.exec(&fold);
        interpolation_values.push(obuf);
    }

}

pub fn assemble_poseidon_circuit(circuit: &mut Circuit<'_, F, Gatebb<'_, F>>) {
    let pi = ExternalValue::new();
    let poseidon_consts = Poseidon::new();

    let mut circuit = Circuit::<F, Gatebb<F>>::new(25, 1);

    let load_pi_advice_head = Advice::new(0,1,1, Rc::new(|_, iext: &[F]| vec![iext[0]]));
    let mut acc = circuit.advice_pub(0, load_pi_advice_head, vec![], vec![&pi])[0];

    println!("Circuit spawned, starting to assemble poseidons.");

    for _ in 0..1000 {
        acc = poseidon_gadget(&mut circuit, &poseidon_consts, 2, 0, vec![acc]);
    }

    circuit.finalize();

    println!("Construction ready. Executing...");

    // FIXME: use criterion's randomness
    pi.set(F::random(OsRng)).unwrap();
    circuit.execute(0);

    println!("Validating witness...");
    circuit.cs.valid_witness();

    circuit
}

pub fn poseidons_pseudo_fold(c: &mut Criterion) {
    let pi = ExternalValue::new();
    let poseidon_consts = Poseidon::new();

    let mut circuit = Circuit::<F, Gatebb<F>>::new(25, 1);

    let load_pi_advice_head = Advice::new(0,1,1, Rc::new(|_, iext: &[F]| vec![iext[0]]));
    let mut acc = circuit.advice_pub(0, load_pi_advice_head, vec![], vec![&pi])[0];

    println!("Circuit spawned, starting to assemble poseidons.");

    for _ in 0..1000 {
        acc = poseidon_gadget(&mut circuit, &poseidon_consts, 2, 0, vec![acc]);
    }

    circuit.finalize();

    println!("Construction ready. Executing...");

    // FIXME: use criterion's randomness
    pi.set(F::random(OsRng)).unwrap();
    circuit.execute(0);

    println!("Validating witness...");
    circuit.cs.valid_witness();

    println!("Preparing homogenized gates...");

    let mu = F::random(OsRng); // relaxation factor

    let mut bench_data = Vec::<(Gatebb<F>, Vec<F>, Vec<F>)>::new();
    for cg in &circuit.cs.cs.cs {
        for constr in &cg.entries {
            let gate = homogenize(constr.gate.clone(), mu);

            let a: Vec<_> = repeat_with(|| F::random(OsRng)).take(gate.i()).collect();
            let b: Vec<_> = repeat_with(|| F::random(OsRng)).take(gate.i()).collect();

            bench_data.push((gate, a, b));
        }
    }

    println!("Bench data prepared! Starting fold evaluation now.");
    println!("Will evaluate {} gates.", bench_data.len());

    c.bench_function("poseidons pseudo fold", |b| b.iter(|| {
        bench_data.iter().for_each(|(gate, a, b)| evaluate_on_random_linear_combinations(gate, a, b))
    }));

}

pub fn poseidons_msm(c: &mut Criterion) {
    let pi = ExternalValue::new();
    let poseidon_consts = Poseidon::new();

    let mut circuit = Circuit::<F, Gatebb<F>>::new(25, 1);

    let load_pi_advice_head = Advice::new(0,1,1, Rc::new(|_, iext: &[F]| vec![iext[0]]));
    let mut acc = circuit.advice_pub(0, load_pi_advice_head, vec![], vec![&pi])[0];

    println!("Circuit spawned, starting to assemble poseidons.");

    for _ in 0..1000 {
        acc = poseidon_gadget(&mut circuit, &poseidon_consts, 2, 0, vec![acc]);
    }

    circuit.finalize();

    println!("Construction ready. Executing...");

    // FIXME: use criterion's randomness
    pi.set(F::random(OsRng)).unwrap();
    circuit.execute(0);

    println!("Validating witness...");
    circuit.cs.valid_witness();

    let witness_len: usize = circuit.cs.wtns.iter().map(|rw| rw.privs.len() + rw.pubs.len()).sum();
    println!("Witness valid. Length: {}", witness_len);

    let mut ck = Vec::with_capacity(circuit.cs.wtns.len());
    for rw in &circuit.cs.wtns {
        let rck: CkRound<G> = rw.privs.iter().map(|_| G::random(OsRng)).collect();
        ck.push(rck);
    }

    c.bench_function("poseidons msm", |b| b.iter(|| circuit.cs.commit(&ck)));
}

criterion_group!(benches, poseidons_pseudo_fold, poseidons_msm);
criterion_main!(benches);