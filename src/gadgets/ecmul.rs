// Elliptic curve operations for variable base.
// Strategy:
// Take a prover-provided random shift Z, check that it is on curve.
// Compute all multiplicities of A for every bitstring from 0 to 2^k - 1, shifted by 2^k Z
// Then, sequentially multiply accumulator by 2^k, and add the multiplicity, conditionally chosen from the chunk.

use std::{rc::Rc, marker::PhantomData};

use ff::{Field, PrimeField};
use halo2curves::{bn256, serde::SerdeObject, CurveExt};
use num_traits::pow;
use crate::{circuit::{Circuit, PolyOp, Advice}, constraint_system::Variable, gate::{Gatebb, RootsOfUnity}};

/// A nonzero elliptic curve point.
pub struct EcAffinePoint<F: PrimeField+RootsOfUnity, C: CurveExt<Base = F>> {
    pub x: Variable,
    pub y: Variable,
    _marker: PhantomData<C>,
}

impl<F: PrimeField+RootsOfUnity, C: CurveExt<Base=F>> EcAffinePoint<F, C> {

    pub fn new_unchecked(x: Variable, y: Variable) -> Self {
        Self{x,y, _marker: PhantomData::<C>}
    }

    pub fn new<'a>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, x: Variable, y: Variable) -> Self{
        circuit.constrain(&[x,y], Gatebb::new(3, 2, 1, Rc::new(|args|{
            let x = args[0];
            let y = args[1];

            let a = C::a();
            let b = C::b();

            vec![x.cube() + a*x + b - y*y]
        })));

        Self::new_unchecked(x,y)
    }
}

// Formulas taken from here: https://en.wikibooks.org/wiki/Cryptography/Prime_Curve/Standard_Projective_Coordinates
// Should later switch to more efficient multiplication by integers (multiplying by F::from(integer) is not efficient),
// not sure if there is an API for that.
pub fn double_internal<F: PrimeField+RootsOfUnity, C: CurveExt<Base=F>>(state: &mut (F,F,F)) {
    let x = state.0;
    let y = state.1;
    let z = state.2;

    let w = F::from(3)*x.square();
    let s = y*z;
    let b = x*y*s;
    let h = w.square() - F::from(8)*b;
    
    state.0 = F::from(2)*h*s;
    let s_sq = s.square();
    state.1 = w*(F::from(4)*b - h) - F::from(8)*y.square()*s_sq;
    state.2 = F::from(8)*s*s_sq;
}

pub fn double_k_times_internal<F: PrimeField+RootsOfUnity, C: CurveExt<Base=F>>(x: F, y: F, k:usize) -> (F, F, F) {
    assert!(C::a() == F::ZERO, "EC ops are implemented only for a=0");
    let mut state = (x,y,F::ONE);
    for _ in 0..k {
        double_internal::<F, C>(&mut state);
    }
    state
}

/// Takes an EC affine point and doubles it k times.
/// Because doubling of a point is never a zero point of a curve, 3rd projective coordinate is always nonzero.
pub fn double_k_times_gadget<'a, F: PrimeField+RootsOfUnity, C: CurveExt<Base=F>>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, k: usize, round: usize, pt: EcAffinePoint<F, C>) -> EcAffinePoint<F,C> {

    let tmp = circuit.advice(
        round,
        Advice::new(2,0,2, Rc::new(move |args, _|{
            let x = args[0];
            let y = args[1];
            let proj_ret = double_k_times_internal::<F, C>(x, y, k);
            let inv_scale = proj_ret.2.invert().unwrap();
            vec![proj_ret.0*inv_scale, proj_ret.1*inv_scale]
        })),
        vec![pt.x, pt.y],
        vec![]
    );

    let ret_x = tmp[0];
    let ret_y = tmp[1];

    circuit.constrain(
        &vec![pt.x, pt.y, ret_x, ret_y],
        Gatebb::new(pow(6,k)+1, 4, 2, Rc::new(move |args|{
            let ptx = args[0];
            let pty = args[1];
            let rhs_x = args[2];
            let rhs_y = args[3];

            let (lhs_x, lhs_y, lhs_z) = double_k_times_internal::<F, C>(ptx, pty, k);
            vec![lhs_x - rhs_x*lhs_z, lhs_y - rhs_y*lhs_z]
        }))
    );

    EcAffinePoint::new_unchecked(ret_x, ret_y)
}