// Implementation taken from arnaucube's poseidon-rs implementation and adapted as blackbox-gadget.
// Also adapted structures so they work with my field.

use std::rc::Rc;

use ff::{Field, PrimeField};
use crate::gadgets::poseidon_constants;
use halo2curves::{bn256, serde::SerdeObject};

use crate::{circuit::{Circuit, PolyOp}, constraint_system::Variable, gate::Gatebb};


type F = bn256::Fr;

pub struct Constants {
    pub c: Vec<Vec<F>>,
    pub m: Vec<Vec<Vec<F>>>,
    pub n_rounds_f: usize,
    pub n_rounds_p: Vec<usize>,
}
pub fn load_constants() -> Constants {
    let (c_str, m_str) = poseidon_constants::constants();
    let mut c: Vec<Vec<F>> = Vec::new();
    for i in 0..c_str.len() {
        let mut cci: Vec<F> = Vec::new();
        for j in 0..c_str[i].len() {
            let b: F = F::from_str_vartime(c_str[i][j]).unwrap();
            cci.push(b);
        }
        c.push(cci);
    }
    let mut m: Vec<Vec<Vec<F>>> = Vec::new();
    for i in 0..m_str.len() {
        let mut mi: Vec<Vec<F>> = Vec::new();
        for j in 0..m_str[i].len() {
            let mut mij: Vec<F> = Vec::new();
            for k in 0..m_str[i][j].len() {
                let b: F = F::from_str_vartime(m_str[i][j][k]).unwrap();
                mij.push(b);
            }
            mi.push(mij);
        }
        m.push(mi);
    }
    Constants {
        c,
        m,
        n_rounds_f: 8,
        n_rounds_p: vec![
            56, 57, 56, 60, 60, 63, 64, 63, 60, 66, 60, 65, 70, 60, 64, 68,
        ],
    }
}


pub struct Poseidon {
    constants: Constants,
}

pub fn new() -> Poseidon {
    Poseidon {
        constants: load_constants(),
    }
}
fn ark(one: F, state: &mut Vec<F>, c: &Vec<F>, it: usize) -> (){
    for i in 0..state.len() {
        state[i] += &c[it+i]*one;
    }
}

fn sbox(one4: F, n_rounds_f: usize, n_rounds_p: usize, state: &mut Vec<F>, i: usize) -> () {
    if i < n_rounds_f / 2 || i >= n_rounds_f / 2 + n_rounds_p {
        for j in 0..state.len() {
            let aux = state[j];
            state[j].square();
            state[j].square();
            state[j] *= &aux;
        }
    } else {
        let aux = state[0];
        state[0].square();
        state[0].square();
        state[0] *= &aux;

        for j in 1..state.len() {
            state[j] *= one4;
        }  
    }
}

fn mix(state: &Vec<F>, m: &Vec<Vec<F>>) -> Vec<F> {
    let mut new_state: Vec<F> = Vec::new();
    for i in 0..state.len() {
        new_state.push(F::zero());
        for j in 0..state.len() {
            let mut mij = m[i][j];
            mij *= &state[j];
            new_state[i] += &mij;
        }
    }
    new_state
}

/// A polynomial operation executing k rounds of Poseidon. Recommended k = 2, which amounts to the polyop of degree 25.
/// Does not make any sanity checks on state length.
fn poseidon_kround_poly(
    k: usize,
    one: F,
    state: &[F],
    i: usize,
    c: &Vec<F>,
    m: &Vec<Vec<F>>,
    n_rounds_f: usize,
    n_rounds_p: usize,
    t: usize,
    ) -> Vec<F> {
    
    let one2 = one*one;
    let one4 = one2*one2;

    let mut state = state.to_vec();
    
    for j in i..i+k {
        ark(one, &mut state, c, j*t);
        sbox(one4, n_rounds_f, n_rounds_p, &mut state, j);
        state = mix(&state, m);
    }

    state
}

pub fn poseidon_gadget<'a>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, cfg: &'a Poseidon, k: usize, round: usize, inp: Vec<Variable>) -> Variable {
    let one = circuit.one();
    let t = inp.len()+1;
    if inp.is_empty() || inp.len() > cfg.constants.n_rounds_p.len() {
        panic!("Wrong inputs length");
    }

    let n_rounds_f = cfg.constants.n_rounds_f;
    let n_rounds_p = cfg.constants.n_rounds_p[t - 2];

    let mut state = circuit.apply(
        round,
        PolyOp::new(
            5*k,
            t-1,
            t,
            Rc::new(
                move |one, inp|{
                    let mut state = vec![F::ZERO; t];
                    state[1..].clone_from_slice(&inp);            
                    poseidon_kround_poly(k, one, &state, 0, &cfg.constants.c[t-2], &cfg.constants.m[t-2], n_rounds_f, n_rounds_p, t)
                }
            )
        ),
        inp);

    for i in 1..(n_rounds_f+n_rounds_p)/k {
        state = circuit.apply(
            round,
            PolyOp::new(
                5*k,
                t,
                t,
                Rc::new(
                    move |one, inp| {
                        poseidon_kround_poly(k, one, inp, i*k, &cfg.constants.c[t-2], &cfg.constants.m[t-2], n_rounds_f, n_rounds_p, t)
                    }
                )
            ),
            state
        )
    }

    let rem = (n_rounds_f + n_rounds_p)%k;

    if rem > 0 {
        state = circuit.apply(
            round,
            PolyOp::new(
                5*rem,
                t,
                t,
                Rc::new(
                    move |one, inp| {
                        poseidon_kround_poly(rem, one, inp, (n_rounds_f + n_rounds_p)-rem, &cfg.constants.c[t-2], &cfg.constants.m[t-2], n_rounds_f, n_rounds_p, t)
                    }
                )
            ),
            state
        )

    }

    state[0]
}