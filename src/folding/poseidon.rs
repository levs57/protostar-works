use std::rc::Rc;
use elsa::map::FrozenMap;
use ff::{Field, PrimeField};
use gate_macro::make_gate;
use crate::{circuit::{Advice, Build}};
use halo2::halo2curves::bn256;
use crate::{circuit::{Circuit, PolyOp}, constraint_system::Variable, gate::Gatebb};
use num_traits::pow;
use crate::utils::field_precomp::FieldUtils;
use super::poseidon_constants;


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
    pub constants: Constants,
}

impl Poseidon {
    pub fn new() -> Poseidon {
        Poseidon {
            constants: load_constants(),
        }
    }


    pub fn hash(&self, inp: Vec<F>) -> F {
        let t = inp.len() + 1;
        // if inp.len() == 0 || inp.len() >= self.constants.n_rounds_p.len() - 1 {
        assert!(! (inp.is_empty() || inp.len() > self.constants.n_rounds_p.len()),
            "Wrong inputs length");
        
        let n_rounds_f = self.constants.n_rounds_f.clone();
        let n_rounds_p = self.constants.n_rounds_p[t - 2].clone();

        let mut state = vec![F::ZERO; t];
        state[1..].clone_from_slice(&inp);

        for i in 0..(n_rounds_f + n_rounds_p) {
            ark(&mut state, &self.constants.c[t - 2], i * t);
            sbox(n_rounds_f, n_rounds_p, &mut state, i);
            state = mix(&state, &self.constants.m[t - 2]);
        }

        state[0]
    }
}

pub fn ark(state: &mut Vec<F>, c: &Vec<F>, it: usize) -> (){
    for i in 0..state.len() {
        state[i] += &c[it+i];
    }
}

pub fn sbox(n_rounds_f: usize, n_rounds_p: usize, state: &mut Vec<F>, i: usize) -> () {
    if i < n_rounds_f / 2 || i >= n_rounds_f / 2 + n_rounds_p {
        for j in 0..state.len() {
            let aux = state[j];
            state[j] = state[j].square();
            state[j] = state[j].square();
            state[j] *= &aux;
        }
    } else {
        let aux = state[0];
        state[0] = state[0].square();
        state[0] = state[0].square();
        state[0] *= &aux;
    }
}

pub fn mix(state: &Vec<F>, m: &Vec<Vec<F>>) -> Vec<F> {
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