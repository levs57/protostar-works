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


/// This computes sc*base, (sc+1)*base
fn mul_neighbor_chain_phase<F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(base: (F,F,F), sc: u64) -> ((F,F,F),(F,F,F)) {
    if sc == 1 {return (base, double_proj::<F,C>(base))};
    
    let (x,y) = mul_neighbor_chain_phase::<F,C>(base, sc >> 1);
    if sc % 2 == 0 {
        (double_proj::<F,C>(x), add_proj::<F,C>(x, y))
    } else {
        (add_proj::<F,C>(x,y), double_proj::<F,C>(y))
    }
}

pub fn mul_doubling_phase<F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(base: (F,F,F), sc: u64) -> (F,F,F) {
    if sc == 1 {
        base
    } else if sc%2 == 0 {
        double_proj::<F,C>(mul_doubling_phase::<F,C>(base, sc >> 1))
    } else {
        let (x,y) = mul_neighbor_chain_phase::<F,C>(base, sc >> 1);
        add_proj::<F,C>(x,y)
    }
}

/// This function will compute multiplication of a point by scalar, using projective coordinates and a specific
/// addition chain which guarantees minimal degree of the corresponding polynomial. I do not know the reference
/// for this theorem, so I'll just write it here in comments.
pub fn best_mul_proj<F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(x: F, y:F, sc: u64) -> (F, F, F) {
    assert!(sc > 0, "Should never happen.");
    mul_doubling_phase::<F,C>((x, y, F::ONE), sc)
}

pub fn double_and_add_proj<F: PrimeField+FieldUtils, C: CurveExt<Base=F>> (x:F, y:F, sc: u64) -> (F, F, F) {
    let mut bits = vec![];
    let mut sc = sc;
    while sc>0 {
        bits.push(sc&1);
        sc>>=1;
    }
    let lg = bits.len();
    let pt = (x, y, F::ONE);
    let mut acc = pt;
    for i in 1..lg {
        acc = double_proj::<F,C>(acc);
        if bits[lg-i-1] == 1 {
            acc = add_proj::<F,C>(acc, pt);
        }
    }

    acc
}

pub fn double_and_add_proj_le<F: PrimeField+FieldUtils, C: CurveExt<Base=F>> (x:F, y:F, sc: u64) -> (F, F, F) {
    let mut pow = sc;
    let mut base = (x, y, F::ONE);
    let mut acc = (F::ZERO, F::ONE, F::ZERO);

    while pow > 0 {
        match pow % 2 {
            0 => {
                base = double_proj::<F,C>(base);
                pow >>= 1;
            }
            1 => {
                if acc == (F::ZERO, F::ONE, F::ZERO) {
                    acc = base;
                } else {
                    acc = add_proj::<F,C>(acc, base);
                }
                pow -= 1;
            }
            _ => unreachable!()      
        }
    }
    acc
}

pub fn double_proj_scaled<F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(pt: (F,F,F), scale_pow: u64) -> (F,F,F) {
    let x = pt.0;
    let y = pt.1;
    let z = pt.2;

    let w = x.square().scale(3);
    let s = y*z;
    let b = x*y*s;
    let h = w.square() - b.scale(8);
    
    let s_sq = s.square();

    let scaling = (z.pow([scale_pow])).invert().unwrap();

    (h*s.scale(2)*scaling, (w*(b.scale(4) - h) - y.square()*s_sq.scale(8))*scaling, (s*s_sq.scale(8))*scaling)
}

pub fn oct_suboptimal<F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(x: F, y: F, deg4: u64, deg2: u64) -> (F,F,F) {
    let pt = (x, y, F::ONE);
    let pt2 = double_proj_scaled::<F,C>(pt, 0);
    let pt4 = double_proj_scaled::<F,C>(pt2, 7);
    let pt8 = double_proj_scaled::<F,C>(pt4, 0);
    let scale4 = pt4.2.pow([deg4]);
    let scale2 = pt2.2.pow([deg2]);
    let scaling = (scale4*scale2).invert().unwrap();
    (pt8.0*scaling, pt8.1*scaling, pt8.2*scaling)
}

pub fn oct_naive<F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(x: F, y: F) -> (F,F,F) {
    let pt = (x, y, F::ONE);
    let pt2 = double_proj_scaled::<F,C>(pt, 0);
    let pt4 = double_proj_scaled::<F,C>(pt2, 0);
    let pt8 = double_proj_scaled::<F,C>(pt4, 0);
    pt8
}

pub fn hex_naive<F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(x: F, y: F) -> (F,F,F) {
    let pt = (x, y, F::ONE);
    let pt2 = double_proj_scaled::<F,C>(pt, 0);
    let pt4 = double_proj_scaled::<F,C>(pt2, 0);
    let pt8 = double_proj_scaled::<F,C>(pt4, 0);
    let pt16 = double_proj_scaled::<F,C>(pt8, 0);
    pt16
}

pub fn quad_aleg_optimal<F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(x: F, y: F) -> (F,F,F) {
    let pt = (x, y, F::ONE);
    let pt2 = double_proj_scaled::<F,C>(pt, 0);
    let pt4 = double_proj_scaled::<F,C>(pt2, 7);
    pt4
}

pub fn sq_aleg_optimal<F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(x: F, y: F) -> (F,F,F) {
    let pt = (x, y, F::ONE);
    let pt2 = double_proj_scaled::<F,C>(pt, 0);
    pt2
}

// Takes an EC affine point and doubles it k times.
// Because doubling of a point is never a zero point of a curve, 3rd projective coordinate is always nonzero.
// pub fn double_k_times_gadget<'a, F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(circuit: &mut Circuit<'a, F, Gatebb<'a, F>>, k: usize, round: usize, pt: EcAffinePoint<F, C>) -> EcAffinePoint<F,C> {

//     let tmp = circuit.advice(
//         round,
//         Advice::new(2,0,2, Rc::new(move |args, _|{
//             let x = args[0];
//             let y = args[1];
//             let proj_ret = double_k_times_internal::<F, C>(x, y, k);
//             let inv_scale = proj_ret.2.invert().unwrap();
//             vec![proj_ret.0*inv_scale, proj_ret.1*inv_scale]
//         })),
//         vec![pt.x, pt.y],
//         vec![]
//     );

//     let ret_x = tmp[0];
//     let ret_y = tmp[1];

//     let d = match k {
//         1 => 6,
//         2 => 30,
//         3 => 174,
//         _ => panic!("Unsupported value of k"),
//     };

//     circuit.constrain(
//         &vec![pt.x, pt.y, ret_x, ret_y],
//         Gatebb::new(d, 4, 2, Rc::new(move |args|{
//             let ptx = args[0];
//             let pty = args[1];
//             let rhs_x = args[2];
//             let rhs_y = args[3];

//             let (lhs_x, lhs_y, lhs_z) = double_k_times_internal::<F, C>(ptx, pty, k);
//             vec![lhs_x - rhs_x*lhs_z, lhs_y - rhs_y*lhs_z]
//         }))
//     );

//     EcAffinePoint::new_unchecked(ret_x, ret_y)
// }