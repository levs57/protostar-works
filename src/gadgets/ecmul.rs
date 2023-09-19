// Elliptic curve operations for variable base.
// Strategy:
// Take a prover-provided random shift Z, check that it is on curve.
// Compute all multiplicities of A for every bitstring from 0 to 2^k - 1, shifted by 2^k Z
// Then, sequentially multiply accumulator by 2^k, and add the multiplicity, conditionally chosen from the chunk.

use std::rc::Rc;

use ff::{Field, PrimeField};
use halo2curves::{bn256, serde::SerdeObject};
use crate::{circuit::{Circuit, PolyOp}, constraint_system::Variable, gate::Gatebb};


type F = bn256::Fr;

