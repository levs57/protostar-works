// Elliptic curve operations for variable base.
// Strategy:
// Take a prover-provided random shift Z, check that it is on curve.
// Compute all multiplicities of A for every bitstring from 0 to 2^k - 1, shifted by 2^k Z
// Then, sequentially multiply accumulator by 2^k, and add the multiplicity, conditionally chosen from the chunk.

use std::{rc::Rc, marker::PhantomData};

use ff::PrimeField;
use halo2curves::CurveExt;
use crate::{circuit::Circuit, constraint_system::Variable, gate::Gatebb};
use crate::utils::field_precomp::FieldUtils;

/// A nonzero elliptic curve point.
pub struct EcAffinePoint<F: PrimeField+FieldUtils, C: CurveExt<Base = F>> {
    pub x: Variable,
    pub y: Variable,
    _marker: PhantomData<C>,
}

impl<F: PrimeField+FieldUtils, C: CurveExt<Base=F>> EcAffinePoint<F, C> {

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
pub fn double_proj<F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(pt: (F,F,F)) -> (F,F,F) {
    let x = pt.0;
    let y = pt.1;
    let z = pt.2;

    let w = x.square().scale(3);
    let s = y*z;
    let b = x*y*s;
    let h = w.square() - b.scale(8);
    
    let s_sq = s.square();

    (h*s.scale(2), (w*(b.scale(4) - h) - y.square()*s_sq.scale(8)), (s*s_sq.scale(8)))
}

/// Addition in projective coordinates. Will fail if a==b.
pub fn add_proj<F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(pt1: (F,F,F), pt2: (F,F,F)) -> (F, F, F){
    let u1 = pt2.1*pt1.2;
    let u2 = pt1.1*pt2.2;
    let v1 = pt2.0*pt1.2;
    let v2 = pt1.0*pt2.2;

    let u = u1 - u2;
    let v = v1 - v2;
    
    let w = pt1.2*pt2.2;

    let vsq = v.square();
    let vcb = vsq*v;

    let a = u.square()*w - vcb - vsq*v2.scale(2);

    (v*a,
    (u*(vsq*v2 - a) - vcb*u2),
    vcb*w,)
}
