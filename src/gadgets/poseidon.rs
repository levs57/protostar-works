// Poseidon gadget.
// Implementation taken from arnaucube's poseidon-rs implementation and adapted as blackbox-gadget.
// Also adapted structures so they work with my field.

use std::rc::Rc;
use elsa::map::FrozenMap;
use ff::{Field, PrimeField};
use gate_macro::make_gate;
use crate::{circuit::{Advice}, folding::{poseidon::{ark, mix, sbox, Poseidon}, poseidon_constants}};
use halo2::halo2curves::bn256;
use crate::{circuit::{Circuit, PolyOp}, constraint_system::Variable, gate::Gatebb};
use num_traits::pow;


type F = bn256::Fr;

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

#[make_gate]
pub fn poseidon_partial_rounds_gate<'c>(n_rounds_p: usize, n_rounds_f: usize, t: usize) -> Gatebb<'c, F> {
    let (c_str, m_str) = poseidon_constants::constants();
    let c: Vec<F> = c_str[t - 2].iter().map(|s| {F::from_str_vartime(s).unwrap()}).collect();
    let m: Vec<Vec<F>> = m_str[t - 2].iter().map(|r| r.iter().map(|s| {F::from_str_vartime(s).unwrap()}).collect()).collect();
    Gatebb::new(
        5,
        2*n_rounds_p+2*t,
        n_rounds_p+t,
        Rc::new(move|args, _|{
            let (tmp, io) = args.split_at(2*n_rounds_p);
            let (adv_in, adv_out) = tmp.split_at(n_rounds_p);
            let (inp, out) = io.split_at(t);
            poseidon_partial_rounds_constraint(inp, out, adv_in, adv_out, &c, &m, n_rounds_f, n_rounds_p, t)
        }), 
        vec![]
    )
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
            move |input_state, _| poseidon_partial_rounds_advice(input_state, c, m, n_rounds_f, n_rounds_p, t)
        ),
        inp.clone(),
    );

    let (adv, out) = tmp.split_at(n_rounds_p);

    // repeat intermediate values twice, then append input and output
    let to_constrain : Vec<Variable> = adv.iter().chain(adv.iter()).chain(inp.iter()).chain(out.iter()).map(|x|*x).collect();

    circuit.constrain_with(
        &to_constrain,
        &poseidon_partial_rounds_gate(n_rounds_p, n_rounds_f, t)
    );

    out.to_vec()
}

pub fn poseidon_full_rounds_gadget<'a>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, cfg: &'a Poseidon, k: usize, round: usize, inp: Vec<Variable>, start: usize, finish: usize) -> Vec<Variable> {

    let t = if start==0 {inp.len()+1} else {inp.len()};

    let n_rounds_f = cfg.constants.n_rounds_f;
    let n_rounds_p = cfg.constants.n_rounds_p[t - 2];

    assert!(start<finish, "Must give positive range.");
    assert!(finish <= n_rounds_f/2 || start >= n_rounds_f/2 + n_rounds_p, "Range intersects partial rounds region.");


    let rem = (finish-start)%k;

    let mut state = inp.clone(); 
    let mut i = start;

    if i == 0 {
        state = circuit.apply(
            round,
            PolyOp::new(
                pow(5,k),
                t-1,
                t,
                move |inp, _| {
                    let mut state = vec![F::ZERO; t];
                    state[1..].clone_from_slice(&inp);            
                    poseidon_kround_poly(k, &state, 0, &cfg.constants.c[t-2], &cfg.constants.m[t-2], n_rounds_f, n_rounds_p, t)
                }
            ),
            state,
        );
        i+=k;
    }

    while i < finish {
        state = circuit.apply(
            round,
            PolyOp::new(
                pow(5,k),
                t,
                t,
                move |inp, _| {
                    poseidon_kround_poly(k, inp, i, &cfg.constants.c[t-2], &cfg.constants.m[t-2], n_rounds_f, n_rounds_p, t)
                }
            ),
            state,
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
                move |inp, _| {
                    poseidon_kround_poly(rem, inp, i, &cfg.constants.c[t-2], &cfg.constants.m[t-2], n_rounds_f, n_rounds_p, t)
                }
            ),
            state,
        )
    }

    state
}

pub fn poseidon_mixed_strategy_start(
    state: &[F],
    i: usize,
    cfg: &Poseidon,
) -> Vec<F> {
    let t = state.len();
    let c = &cfg.constants.c[t-2];
    let m = &cfg.constants.m[t-2];
    let n_rounds_f = cfg.constants.n_rounds_f;
    let n_rounds_p = cfg.constants.n_rounds_p[t-2];
    let mut state = state.to_vec();

    ark(&mut state, c, t*i);
    sbox(n_rounds_f, n_rounds_p, &mut state, i);
    state = mix(&state, m);
    ark(&mut state, c, t*(i+1)); // The head of i+1-st round.

    state
}

pub fn poseidon_mixed_strategy_mid(
    state: &[F],
    i: usize,
    cfg: &Poseidon,
) -> Vec<F> {
    let t = state.len();
    let c = &cfg.constants.c[t-2];
    let m = &cfg.constants.m[t-2];
    let n_rounds_f = cfg.constants.n_rounds_f;
    let n_rounds_p = cfg.constants.n_rounds_p[t-2];
    let mut state = state.to_vec();

    
    sbox(n_rounds_f, n_rounds_p, &mut state, i);
    state = mix(&state, m);
    ark(&mut state, c, t*(i+1)); // The head of i+1-st round.
    sbox(n_rounds_f, n_rounds_p, &mut state, i+1);

    state
}

pub fn poseidon_mixed_strategy_end(
    state: &[F],
    i: usize,
    cfg: &Poseidon,
) -> Vec<F> {
    let t = state.len();
    let c = &cfg.constants.c[t-2];
    let m = &cfg.constants.m[t-2];
    let n_rounds_f = cfg.constants.n_rounds_f;
    let n_rounds_p = cfg.constants.n_rounds_p[t-2];
    let mut state = state.to_vec();

    state = mix(&state, m); // The tail of i-1 st round.
    ark(&mut state, c, t*i);
    sbox(n_rounds_f, n_rounds_p, &mut state, i);
    mix(&state, m) // Ends in i-th round
}

pub fn poseidon_mixed_strategy_full_rounds_gadget<'a>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, cfg: &'a Poseidon, round: usize, state: Vec<Variable>, is_first_part:bool) -> Vec<Variable>{
    let t = if is_first_part {state.len() + 1} else {state.len()};
    
    let n_rounds_f = cfg.constants.n_rounds_f;
    let n_rounds_p = cfg.constants.n_rounds_p[t-2];

    assert!(n_rounds_f == 8, "This should never fail.");
    
    let i = if is_first_part {0} else {n_rounds_f/2 + n_rounds_p};
    
    let mut state = if is_first_part {
        circuit.apply(round,
            PolyOp::new(
                5,
                t-1,
                t,
                move |inp, _| {
                    let mut state = vec![F::ZERO; t];
                    state[1..].clone_from_slice(&inp);
                    poseidon_mixed_strategy_start(&state, i, cfg)
                }
            ),
            state,
        )
    } else {
        circuit.apply(round,
            PolyOp::new(
                5,
                t,
                t,
                move |state, _| {
                    poseidon_mixed_strategy_start(state, i, cfg)
                }
            ),
            state,
        )
    };

    state = circuit.apply(
        round,
        PolyOp::new(
            25,
            t,
            t,
            move |state, _| poseidon_mixed_strategy_mid(state, i + 1, cfg)
        ),
        state,
    );

    circuit.apply(
        round,
        PolyOp::new(
            5,
            t,
            t,
            move |state, _| poseidon_mixed_strategy_end(state, i + 3, cfg)
        ),
        state,
    )
}

pub fn poseidon_gadget_internal<'a>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, cfg: &'a Poseidon, k: usize, round: usize, inp: Vec<Variable>) -> Variable {
    let t = inp.len()+1;
    if inp.is_empty() || inp.len() > cfg.constants.n_rounds_p.len() {
        panic!("Wrong inputs length");
    }

    let n_rounds_f = cfg.constants.n_rounds_f;
    let n_rounds_p = cfg.constants.n_rounds_p[t - 2];

    assert!(k < n_rounds_f/2, "Not implemented for k larger than half of the full rounds. Also, you shouldn't do this anyways.");

    let mut state = poseidon_full_rounds_gadget(circuit, cfg, k, round, inp, 0, n_rounds_f/2);
    state = poseidon_partial_rounds_gadget(circuit, cfg, state, round);
    state = poseidon_full_rounds_gadget(circuit, cfg, k, round, state, n_rounds_f/2 + n_rounds_p, n_rounds_f + n_rounds_p);
    state[0]
}

pub fn poseidon_gadget_mixstrat<'a>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, cfg: &'a Poseidon, round: usize, inp: Vec<Variable>) -> Variable {
    let mut state = poseidon_mixed_strategy_full_rounds_gadget(circuit, cfg, round, inp, true);
    state = poseidon_partial_rounds_gadget(circuit, cfg, state, round);
    state = poseidon_mixed_strategy_full_rounds_gadget(circuit, cfg, round, state, false);
    state[0]
}

/// Hashes an array with some rate. Recommended rate is (allegedly) around 10; need to check whether evaluation of
/// linear matrices becomes too slow (might also explore Neptune strategy, which is very similar to what we are doing).
pub fn poseidon_gadget<'a>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, cfg: &'a Poseidon, round: usize, rate: usize, inp: &[Variable]) -> Variable {
    let k = 1;
    let l = inp.len();
    assert!(l>0, "Can not hash empty array without padding.");
    let mut to_hash = vec![];
    
    for i in 0..l {
        to_hash.push(inp[l-i-1]) // revert order so it is more convenient to pop stuff
    }

    while to_hash.len()>1 {
        let mut chunk = vec![];
        while to_hash.len()>0 && chunk.len() < rate {
            chunk.push(to_hash.pop().unwrap());
        }
        to_hash.push(poseidon_gadget_internal(circuit, cfg, k, round, chunk));
    }

    to_hash[0]
}