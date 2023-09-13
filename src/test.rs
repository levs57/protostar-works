use std::{rc::Rc, cell::{RefCell, Cell}, iter::repeat};

use crate::{gate::{self, RootsOfUnity, Gatebb, Gate}, constraint_system::Variable, circuit::{Circuit, ExternalValue, PolyOp, Advice}, gadgets::{poseidon::{poseidon_gadget, Poseidon, ark, sbox, mix, poseidon_kround_poly}, bits::bit_decomposition_gadget}};
use ff::{PrimeField, Field};
use halo2::arithmetic::best_fft;
use halo2curves::{bn256, serde::SerdeObject};
use num_traits::pow;
use rand_core::OsRng;

type F = bn256::Fr;

impl RootsOfUnity for F {
    /// Returns power of a primitive root of unity of order 2^logorder.
    fn roots_of_unity(power: u64, logorder: usize) -> Self{
        F::ROOT_OF_UNITY.pow([pow(2, F::S as usize - logorder)]).pow([power])
    }
    /// Returns power of 1/2.
    fn half_pow(power: u64) -> Self {
        F::TWO_INV.pow([power])
    }

    fn binomial_FFT(power: usize, logorder: usize) -> Vec<Self> {
        assert!(power < pow(2, logorder));
        let mut bin_coeffs = vec![1];
        for i in 1..pow(2,logorder) {
            let tmp = bin_coeffs[i-1];
            // n!/((i-1)!(n-i+1)!) * (n-i)/i
            if i <= power{
                bin_coeffs.push((tmp * (power-i+1)) / i)
            } else {
                bin_coeffs.push(0)
            }
        }
        let mut bin_coeffs : Vec<F>= bin_coeffs.iter().map(|x|F::from(*x as u64)).collect();
        let omega = F::roots_of_unity(1, logorder);
        best_fft(&mut bin_coeffs, omega, logorder as u32);
        bin_coeffs
    }
}

#[test]

fn test_cross_terms() {

    for d in 2..10{
        let f = Rc::new(|v: &[F]| vec![v[0].pow([2 as u64])]);
        let gate = Gatebb::new(2, 1, 1, f);
        let tmp = gate.cross_terms_adjust(&vec![F::ONE], &vec![F::ONE], d);
        println!("{:?}", tmp.iter().map(|v|v[0]).collect::<Vec<_>>());
    }
}

#[test]

fn test_circuit_builder() {
    let public_input_source = ExternalValue::<F>::new();

    let mut circuit = Circuit::<F, Gatebb<F>>::new(2, 1);

    let sq = PolyOp::new(2, 1, 1, Rc::new(|_: F, x|vec!(x[0]*x[0])));
    let input = circuit.advice_pub(0, Advice::new(0, 1, 1, Rc::new(|_, iext|vec![iext[0]])), vec![], vec![&public_input_source])[0];
    let sq1 = circuit.apply(0, sq.clone(), vec![input]);
    let output = circuit.apply_pub(0, sq.clone(), sq1);

    circuit.finalize();

    public_input_source.set(F::from(2)).unwrap();

    circuit.execute(0);

    let circuit2 = circuit.clone();

    println!("{:?}", circuit.cs.getvar(Variable::Public(0,2)).to_repr());
}

#[test]

fn test_permutation_argument() {
    let pi_ext : Vec<_> = repeat(ExternalValue::<F>::new()).take(5).collect();
    let challenge_ext = ExternalValue::<F>::new();

    let mut circuit = Circuit::<F, Gatebb<F>>::new(2, 2);
    
    let one = Variable::Public(0,0);

    let read_pi_advice = Advice::new(0,1,1, Rc::new(|_, iext: &[F]| vec![iext[0]]));
    

    let mut pi = vec![];
    for k in 0..5{
        pi.push(
            circuit.advice_pub(0, read_pi_advice.clone(), vec![], vec![&pi_ext[k]])[0]
        );
    }

    let challenge = circuit.advice_pub(1, read_pi_advice.clone(), vec![], vec![&challenge_ext])[0];

    let division_advice = Advice::new(2, 0, 1, Rc::new(|ivar : &[F], _| {
        let ch = ivar[0];
        let x = ivar[1];
        vec![(x-ch).invert().unwrap()]
    }));

    let mut fractions = vec![];
    for k in 0..5 {
        fractions.push(
            circuit.advice(1, division_advice.clone(), vec![challenge, pi[k]], vec![])[0]
        );
    }

    let div_constr = Gatebb::<F>::new(2, 4, 1, Rc::new(|args|{
        let one = args[0];
        let ch = args[1];
        let x = args[2];
        let res = args[3];
        vec![one*one - res * (x-ch)]
    }));

    for k in 0..5 {
        circuit.constrain(&[one, challenge, pi[k], fractions[k]], div_constr.clone());
    }

    circuit.finalize();

    // construction phase ended

    pi_ext[0].set(F::from(2)).unwrap();
    pi_ext[1].set(F::from(3)).unwrap();
    pi_ext[2].set(F::from(4)).unwrap();
    pi_ext[3].set(F::from(5)).unwrap();
    pi_ext[4].set(F::from(6)).unwrap();

    circuit.execute(0);

    challenge_ext.set(F::random(OsRng)).unwrap();
    circuit.execute(1);

    circuit.cs.valid_witness(); // test that constraints are satisfied
}

#[test]
fn test_poseidon_gadget(){
    let cfg = Poseidon::new();
    let pi_ext = ExternalValue::<F>::new();
    let mut circuit = Circuit::<F, Gatebb<F>>::new(25, 1);
    let read_pi_advice = Advice::new(0,1,1, Rc::new(|_, iext: &[F]| vec![iext[0]]));    
    let pi = circuit.advice_pub(0, read_pi_advice.clone(), vec![], vec![&pi_ext])[0];
    let ret = poseidon_gadget(&mut circuit, &cfg, 1, 0, vec![pi]);

    circuit.finalize();

    pi_ext.set(F::ONE).unwrap();

    circuit.execute(0);
    circuit.cs.valid_witness();

    assert!(circuit.cs.getvar(ret) == F::from_str_vartime("18586133768512220936620570745912940619677854269274689475585506675881198879027").unwrap());

    println!("{:?}", circuit.cs.getvar(ret).to_repr());
}

#[test]

fn test_bit_decomposition(){
    let pi_ext = ExternalValue::<F>::new();
    let mut circuit = Circuit::<F, Gatebb<F>>::new(2, 1);
    let read_pi_advice = Advice::new(0,1,1, Rc::new(|_, iext: &[F]| vec![iext[0]]));    
    let pi = circuit.advice_pub(0, read_pi_advice.clone(), vec![], vec![&pi_ext])[0];

    let bits = bit_decomposition_gadget(&mut circuit, 0, 3, pi);

    circuit.finalize();
    pi_ext.set(F::from(6)).unwrap();
    circuit.execute(0);

    circuit.cs.valid_witness();

    assert!(bits.len()==3);
    assert!(circuit.cs.getvar(bits[0]) == F::ZERO);
    assert!(circuit.cs.getvar(bits[1]) == F::ONE);
    assert!(circuit.cs.getvar(bits[2]) == F::ONE);
}