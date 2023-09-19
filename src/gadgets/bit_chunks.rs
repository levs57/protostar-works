// Decomposition into multibit-chunks without using lookup.
// Dubious efficiency, requires more research.
// There is also an optimization allowing to improve computation speed of a polynomial
// x(x-1)(x-2)...(x-2^k+1) ~roughly 2x which I didn't implement currently
// Generally, I recommend using relatively low value of k here - the evaluation time
// Of this polynomial is comparable with its degree.


use std::{rc::Rc, iter::repeat};

use ff::PrimeField;
use num_traits::pow;

use crate::{circuit::{Circuit, Advice, PolyOp}, gate::{Gatebb, RootsOfUnity}, constraint_system::Variable};

pub fn chunkcheck<F: PrimeField>(arg: &[F], k:usize) -> Vec<F> {
    let x = arg[0];
    let mut acc = x;
    for i in 1..pow(2,k) {
        acc *= x-F::from(i as u64); 
    }
    vec![acc]
}

pub fn decompcheck<F: PrimeField>(arg: &[F], k: usize) -> Vec<F> {
    let x = arg[0];
    let shift = F::from(pow(2,k));
    let mut acc = F::ZERO;
    for i in 1..arg.len() {
        acc *= shift;
        acc += arg[arg.len()-i];
    }
    vec![acc-x]

}

pub fn decompose<F: PrimeField>(x: F, k: usize, num_chunks: usize) -> Vec<F> {
    let x = x.to_repr();
    let mut chunks = vec![]; 
    for i in 0..x.as_ref().len() {
        for j in 0..8 {
            let s = (j+i*8) as usize;
            if s % k == 0 {
                chunks.push(0);
            }
            let tmp = chunks.len() - 1;
            chunks[tmp] += pow(2,s%k) * ((x.as_ref()[i]>>j)&1)
        }
    }
    for i in num_chunks..chunks.len() {
        assert!(chunks[i] == 0, "The scalar is too large.");
    }

    chunks.iter().take(num_chunks).map(|x|F::from(*x as u64)).collect()
}

pub fn bit_chunks_gadget<'a, F: PrimeField+RootsOfUnity>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, round: usize, num_chunks: usize, chunk_size: usize, input: Variable) -> Vec<Variable> {
    let k = chunk_size;
    let chunks = circuit.advice(
        round,
        Advice::new(1, 0, num_chunks, Rc::new(move |input,_|{
            let input = input[0];
            decompose(input, k, num_chunks)
        })),
        vec![input],
        vec![]
    );

    let chunkcheck_gate = Gatebb::new(pow(2,k), 1, 1, Rc::new(move |chunk: &[F]|{
            chunkcheck(chunk, k)
        }
    ));

    for i in 0..num_chunks-1 {
        circuit.constrain(&vec![chunks[i]], chunkcheck_gate.clone())
    }
    circuit.constrain(&vec![chunks[num_chunks-1]], chunkcheck_gate);

    let decompcheck_gate = Gatebb::new(1, num_chunks+1, 1, Rc::new(move|x|decompcheck::<F>(x,k)));

    let tmp : Vec<_> = repeat(input).take(1).chain(chunks.iter().map(|x|*x)).collect();
    circuit.constrain(&tmp, decompcheck_gate);
    
    chunks
}