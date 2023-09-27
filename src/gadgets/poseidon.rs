// Poseidon gadget.
// Implementation taken from arnaucube's poseidon-rs implementation and adapted as blackbox-gadget.
// Also adapted structures so they work with my field.

use std::rc::Rc;

use ff::{Field, PrimeField};
use crate::{gadgets::poseidon_constants, circuit::Advice};
use halo2curves::{bn256, serde::SerdeObject};
use crate::{circuit::{Circuit, PolyOp}, constraint_system::Variable, gate::Gatebb};
use num_traits::pow;


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

/// A polynomial operation executing k rounds of Poseidon. Recommended k = 2, which amounts to the polyop of degree 25.
/// Does not make any sanity checks on state length.
pub fn poseidon_kround_poly(
    k: usize,
    state: &[F],
    i: usize,
    c: &Vec<F>,
    m: &Vec<Vec<F>>,
    n_rounds_f: usize,
    n_rounds_p: usize,
    t: usize,
    ) -> Vec<F> {

    let mut state = state.to_vec();
    
    for j in 0..k {
        ark(&mut state, c, (i+j)*t);
        sbox(n_rounds_f, n_rounds_p, &mut state, i+j);
        state = mix(&state, m);
    }

    state
}

/// This function is a single polynomial of degree 5, executing all partial rounds in one go.
/// It takes as an advice the vector of 1st elements of each state after the sbox operation.
/// It is given in a form of constraint, because otherwise we would need to introduce an additional linear constraint.
/// This is an API limitation - currently there is no way to map output of a polynomial operation to an already
/// existing variable.
/// Input advices correspond to 5-th power of the state[0], and output advices to new state[0].
/// User then must map them to the same variable input.
pub fn poseidon_partial_rounds_constraint(
    input_state: &[F],
    output_state: &[F],
    input_advices: &[F],
    output_advices: &[F],
    c: &Vec<F>,
    m: &Vec<Vec<F>>,
    n_rounds_f: usize,
    n_rounds_p: usize,
    t: usize,
) -> Vec<F> {
    let mut state = input_state.to_vec();
    let mut ret = vec![];
    for j in 0 .. n_rounds_p {
        ark(&mut state, c, t*(j+n_rounds_f/2));
        let tmp = state[0].square().square();
        ret.push(state[0]*tmp - output_advices[j]); // push state[0]^5 == output_advices[j]
        state[0] = input_advices[j]; // replace value with input advice
        state = mix(&state, m);
    }

    for j in 0 .. state.len(){
        ret.push(state[j] - output_state[j]);
    }

    ret
}

/// This will compute intermediate state[0] values and the final state
pub fn poseidon_partial_rounds_advice(
    input_state: &[F],
    c: &Vec<F>,
    m: &Vec<Vec<F>>,
    n_rounds_f: usize,
    n_rounds_p: usize,
    t: usize,
) -> Vec<F> {
    let mut state = input_state.to_vec();
    let mut ret = vec![];
    for j in 0 .. n_rounds_p {
        ark(&mut state, c, t*(j+n_rounds_f/2));
        let tmp = state[0].square().square();
        state[0] *= tmp;
        ret.push(state[0]);
        state = mix(&state, m);
    }

    for j in 0 .. state.len(){
        ret.push(state[j]);
    }

    ret
}

/// A gadget which implements partial rounds of Poseidon hash function.
pub fn poseidon_partial_rounds_gadget<'a>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, cfg: &'a Poseidon, inp: Vec<Variable>, round: usize) -> Vec<Variable>{
    let t = inp.len();
    let n_rounds_f = cfg.constants.n_rounds_f;
    let n_rounds_p = cfg.constants.n_rounds_p[t - 2];
    let c = &cfg.constants.c[t-2];
    let m = &cfg.constants.m[t-2];

    let tmp = circuit.advice(
        round,
        Advice::new(
            t,
            0,
            n_rounds_p + t,
            Rc::new(move |input_state: &[F], _: &[F]| {
                poseidon_partial_rounds_advice(input_state, c, m, n_rounds_f, n_rounds_p, t)
            })
        ),
        inp.clone(),
        vec![],
    );

    let (adv, out) = tmp.split_at(n_rounds_p);

    // repeat intermediate values twice, then append input and output
    let to_constrain : Vec<Variable> = adv.iter().chain(adv.iter()).chain(inp.iter()).chain(out.iter()).map(|x|*x).collect();

    circuit.constrain(
        &to_constrain,
        Gatebb::new(
            5,
            2*n_rounds_p+2*t,
            n_rounds_p+t,
            Rc::new(move|args: &[F]|{
                let (tmp, io) = args.split_at(2*n_rounds_p);
                let (adv_in, adv_out) = tmp.split_at(n_rounds_p);
                let (inp, out) = io.split_at(t);
                poseidon_partial_rounds_constraint(inp, out, adv_in, adv_out, c, m, n_rounds_f, n_rounds_p, t)
            })
        )
    );

    out.to_vec()
}

pub fn poseidon_full_rounds_gadget<'a>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, cfg: &'a Poseidon, k: usize, round: usize, inp: Vec<Variable>, start: usize, finish: usize) -> Vec<Variable> {
    let t = inp.len();
    let n_rounds_f = cfg.constants.n_rounds_f;
    let n_rounds_p = cfg.constants.n_rounds_p[t - 2];

    assert!(start<finish, "Must give positive range.");
    assert!(finish <= n_rounds_f/2 || start >= n_rounds_f/2 + n_rounds_p, "Range intersects partial rounds region.");

    let rem = (finish-start)%k;

    let mut state = inp.clone(); 
    let mut i = start;

    while i < finish {
        state = circuit.apply(
            round,
            PolyOp::new(
                pow(5,k),
                t,
                t,
                Rc::new(
                    move |inp| {
                        poseidon_kround_poly(k, inp, i, &cfg.constants.c[t-2], &cfg.constants.m[t-2], n_rounds_f, n_rounds_p, t)
                    }
                )
            ),
            state
        );        
        i+=k
    }

    if rem > 0 {
        state = circuit.apply(
            round,
            PolyOp::new(
                pow(5,rem),
                t,
                t,
                Rc::new(
                    move |inp| {
                        poseidon_kround_poly(rem, inp, i, &cfg.constants.c[t-2], &cfg.constants.m[t-2], n_rounds_f, n_rounds_p, t)
                    }
                )
            ),
            state
        )
    }

    state
}

pub fn poseidon_gadget<'a>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, cfg: &'a Poseidon, k: usize, round: usize, inp: Vec<Variable>) -> Variable {
    let t = inp.len()+1;
    if inp.is_empty() || inp.len() > cfg.constants.n_rounds_p.len() {
        panic!("Wrong inputs length");
    }

    let n_rounds_f = cfg.constants.n_rounds_f;
    let n_rounds_p = cfg.constants.n_rounds_p[t - 2];

    assert!(k < n_rounds_f/2, "Not implemented for k larger than half of the full rounds. Also, you shouldn't do this anyways.");

    let mut state = circuit.apply(
        round,
        PolyOp::new(
            pow(5,k),
            t-1,
            t,
            Rc::new(
                move |inp|{
                    let mut state = vec![F::ZERO; t];
                    state[1..].clone_from_slice(&inp);            
                    poseidon_kround_poly(k, &state, 0, &cfg.constants.c[t-2], &cfg.constants.m[t-2], n_rounds_f, n_rounds_p, t)
                }
            )
        ),
        inp);

    state = poseidon_full_rounds_gadget(circuit, cfg, k, round, state, k, n_rounds_f/2);
    state = poseidon_partial_rounds_gadget(circuit, cfg, state, round);
    state = poseidon_full_rounds_gadget(circuit, cfg, k, round, state, n_rounds_f/2 + n_rounds_p, n_rounds_f + n_rounds_p);
    state[0]
}