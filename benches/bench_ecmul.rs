use std::{rc::Rc, iter::repeat_with};

use criterion::{Criterion, criterion_main, criterion_group, black_box};
use ff::Field;
use group::{Group, Curve};
use halo2curves::{bn256, grumpkin, CurveAffine, CurveExt};
use protostar_works::{circuit::{ExternalValue, Circuit, Advice}, gate::{Gatebb, Gate}, gadgets::{ecmul::{EcAffinePoint, escalarmul_gadget_9}, nonzero_check::nonzero_gadget}, utils::poly_utils::bits_le};
use rand_core::OsRng;

type F = bn256::Fr;
type C = grumpkin::G1;

type Fq = <C as CurveExt>::ScalarExt;

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

fn ecmul_pseudo_fold(c: &mut Criterion) {
    let pi_a_ext = (ExternalValue::<F>::new(), ExternalValue::<F>::new());
    let pi_b_ext = (ExternalValue::<F>::new(), ExternalValue::<F>::new()); // a*(1+9+...+9^{nl-1})+b=0 must be checked out of band
    let pi_pt_ext = (ExternalValue::<F>::new(), ExternalValue::<F>::new());
    let pi_sc_ext = ExternalValue::<F>::new();

    let mut circuit = Circuit::<F, Gatebb<F>>::new(10, 1);

    let read_pi = Advice::new(0,1,1, Rc::new(|_, iext: &[F]| vec![iext[0]]));    

    let x = circuit.advice(0, read_pi.clone(), vec![], vec![&pi_a_ext.0])[0];
    let y = circuit.advice(0, read_pi.clone(), vec![], vec![&pi_a_ext.1])[0];
    let a = EcAffinePoint::<F,C>::new(&mut circuit, x, y);

    let x = circuit.advice(0, read_pi.clone(), vec![], vec![&pi_b_ext.0])[0];
    let y = circuit.advice(0, read_pi.clone(), vec![], vec![&pi_b_ext.1])[0];
    let b = EcAffinePoint::<F,C>::new(&mut circuit, x, y);

    let x = circuit.advice(0, read_pi.clone(), vec![], vec![&pi_pt_ext.0])[0];
    let y = circuit.advice(0, read_pi.clone(), vec![], vec![&pi_pt_ext.1])[0];
    let pt = EcAffinePoint::<F,C>::new(&mut circuit, x, y);

    let sc = circuit.advice(0, read_pi.clone(), vec![], vec![&pi_sc_ext])[0];

    let mut nonzeros = vec![];
    let num_limbs = 40;

    let scmul = escalarmul_gadget_9(&mut circuit, sc, pt, num_limbs, 0, a, b, &mut nonzeros);

    nonzero_gadget(&mut circuit, &nonzeros, 9);
    circuit.finalize();

    let pi_a = C::random(OsRng).to_affine();
    pi_a_ext.0.set(pi_a.x).unwrap(); pi_a_ext.1.set(pi_a.y).unwrap();

    //1+9+81+...+9^{num_limbs - 1} = (9^{num_limbs}-1)/8

    let bscale = (Fq::from(9).pow([num_limbs as u64])-Fq::ONE)*(Fq::from(8).invert().unwrap());
    let pi_b = -(C::from(pi_a)*bscale).to_affine();
    pi_b_ext.0.set(pi_b.x).unwrap(); pi_b_ext.1.set(pi_b.y).unwrap();

    let pi_pt = C::random(OsRng).to_affine();
    pi_pt_ext.0.set(pi_pt.x).unwrap(); pi_pt_ext.1.set(pi_pt.y).unwrap();

    pi_sc_ext.set(F::from(23)).unwrap();

    println!("Executing...");

    circuit.execute(0);

    println!("Validating...");

    circuit.cs.valid_witness();

    let answer = grumpkin::G1Affine::from_xy(circuit.cs.getvar(scmul.x), circuit.cs.getvar(scmul.y)).unwrap();

    assert!(answer == (pi_pt*<C as CurveExt>::ScalarExt::from(23)).to_affine());

    println!("Total circuit size: private: {} public: {}", circuit.cs.wtns[0].privs.len(), circuit.cs.wtns[0].pubs.len());

    let mu = F::random(OsRng); // relaxation factor

    let mut bench_data = Vec::<(Gatebb<F>, Vec<F>, Vec<F>, Vec<F>)>::new();
    for cg in &circuit.cs.cs.cs {
        for constr in &cg.entries {
            let gate = homogenize(constr.gate.clone(), mu);

            let a: Vec<_> = repeat_with(|| F::random(OsRng)).take(gate.i()).collect();
            let b: Vec<_> = repeat_with(|| F::random(OsRng)).take(gate.i()).collect();

            let randomness: Vec<_> = repeat_with(|| F::random(OsRng)).take(gate.d()).collect();

            bench_data.push((gate, a, b, randomness));
        }
    }

    c.bench_function("ecmul pseudo fold", |b| b.iter(|| {
        bench_data.iter().for_each(|(gate, a, b, randomness)| black_box(evaluate_on_random_linear_combinations(gate, a, b, randomness)))
    }));
}

criterion_group!(benches, ecmul_pseudo_fold);
criterion_main!(benches);