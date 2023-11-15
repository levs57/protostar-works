// Elliptic curve operations for variable base.
// Strategy:
// Take a prover-provided random shift Z, check that it is on curve.
// Compute all multiplicities of A for every bitstring from 0 to 2^k - 1, shifted by 2^k Z
// Then, sequentially multiply accumulator by 2^k, and add the multiplicity, conditionally chosen from the chunk.

use std::{rc::Rc, marker::PhantomData};

use ff::{PrimeField, BatchInvert};
use halo2::halo2curves::CurveExt;
use crate::circuit::{PolyOp, Advice};
use crate::{circuit::Circuit, constraint_system::Variable, gate::Gatebb};
use crate::utils::field_precomp::FieldUtils;

use super::range::{limb_decompose_gadget, choice_gadget};


#[derive(Clone, Copy)]
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
        circuit.constrain(&[x,y], Gatebb::new(3, 2, 1, Rc::new(|args, _|{
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
pub fn double_proj<F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(pt: (F,F)) -> (F,F,F) {
    let x = pt.0;
    let y = pt.1;

    let y_sq = y.square();

    let w = x.square().scale(3);
    let b = x*y_sq;
    let h = w.square() - b.scale(8);
    
    (h*y.scale(2), (w*(b.scale(4) - h) - y_sq.square().scale(8)), (y*y_sq.scale(8)))
}

/// Addition in projective coordinates. Will fail if a==b.
pub fn add_proj<F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(pt1: (F,F), pt2: (F,F)) -> (F, F, F){
    let u1 = pt2.1;
    let u2 = pt1.1;
    let v1 = pt2.0;
    let v2 = pt1.0;

    let u = u1 - u2;
    let v = v1 - v2;

    let vsq = v.square();
    let vcb = vsq*v;

    let a = u.square() - vcb - vsq*v2.scale(2);

    (v*a,
    (u*(vsq*v2 - a) - vcb*u2),
    vcb)
}

/// Gadget checking that pt1, pt2 and -pt3 points lie in the same line and do not coincide.
pub fn eclin_gadget<'a, F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(
    circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
    pt1: EcAffinePoint<F,C>,
    pt2: EcAffinePoint<F,C>,
    pt3: EcAffinePoint<F,C>, 
    nonzeros: &mut Vec<Variable>,
    round: usize
) -> () {
    let pts = vec![pt1.x, pt1.y, pt2.x, pt2.y, pt3.x, pt3.y];

    circuit.constrain( // Constrain that they are on the same line
        &pts,
        Gatebb::new(
            2,
            6,
            1,
            Rc::new(|args, _|{
                let a = args[2]-args[0];
                let b = args[3]-args[1];
                let c = args[4]-args[0];
                let d = -args[5]-args[1];
                vec![a*d - b*c]
            })
        )
    );

    nonzeros.push(
        circuit.apply(
            round,
            PolyOp::new(
                3,
                3,
                1,
                |args, _| {
                    vec![(args[0]-args[1])*(args[0]-args[2])*(args[1]-args[2])]
                }
            ), 
            vec![pt1.x,pt2.x,pt3.x]
        )[0]
    );
}

/// Gadget which checks that a line passing through a pair of points pt1 and -pt2 is tangent in pt1,
/// and that they do not coincide.
pub fn ectangent_gadget<'a, F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(
    circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
    pt1: EcAffinePoint<F,C>,
    pt2: EcAffinePoint<F,C>,
    nonzeros: &mut Vec<Variable>,
    round: usize
) -> () {
    let pts = vec![pt1.x, pt1.y, pt2.x, pt2.y];
    circuit.constrain( // Check that slope vector is collinear with vector from pt1 to [-pt2]
        &pts,
        Gatebb::new(
            2,
            4,
            1,
            Rc::new(move |args, _ |{
                let a = args[2]-args[0];
                let b = -args[3]-args[1];
                let c = args[1].scale(2);
                let d = args[0].square().scale(3);
                vec![a*d - b*c]
            })
        )
    );
    nonzeros.push(
        circuit.apply(
            round,
            PolyOp::new(
                1,
                3,
                1,
                move |args, _| {
                    vec![args[0]-args[1]]
                }
            ), 
            vec![pt1.x,pt2.x]
        )[0]
    );    
}


/// Addition gadget. If you need batch inversion at witness generation step, use eclin instead,
/// and compute advice separately.
pub fn ecadd_gadget<'a, F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(
    circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
    pt1: EcAffinePoint<F,C>,
    pt2: EcAffinePoint<F,C>,
    nonzeros: &mut Vec<Variable>,
    round: usize
) -> EcAffinePoint<F,C>{
    let tmp = circuit.advice(
        round,
        Advice::new(
            4,
            0,
            2,
            move |args, _| {
                let (x,y,z) = add_proj::<F,C>((args[0], args[1]), (args[2], args[3]));
                let zinv = z.invert().unwrap();
                vec![x*zinv, y*zinv]
            }
        ),
        vec![pt1.x, pt1.y, pt2.x, pt2.y],
        vec![]
    );

    let pt3 = EcAffinePoint::<F,C>::new(circuit, tmp[0], tmp[1]);
    eclin_gadget(circuit, pt1, pt2, pt3, nonzeros, round);
    pt3
}


/// Doubling gadget. If you need batch inversion during witness generation, use ectangent instead
/// and compute advice separately.
pub fn ecdouble_gadget<'a, F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(
    circuit: &mut Circuit<'a, F, Gatebb<'a, F>>,
    pt: EcAffinePoint<F,C>,
    nonzeros: &mut Vec<Variable>,
    round: usize
) -> EcAffinePoint<F,C>{
    let tmp = circuit.advice(
        round,
        Advice::new(
            4,
            0,
            2,
            move |args, _| {
                let (x,y,z) = double_proj::<F,C>((args[0], args[1]));
                let zinv = z.invert().unwrap();
                vec![x*zinv, y*zinv]
            }
        ),
        vec![pt.x, pt.y],
        vec![]
    );

    let pt2 = EcAffinePoint::<F,C>::new(circuit, tmp[0], tmp[1]);
    ectangent_gadget(circuit, pt, pt2, nonzeros, round);
    pt2
}

/// A gadget that multiples a point by a given scalar
/// Prover also must provide a pair of points (a, b) satisfying b + (1+9+9^2+...9^{num_limbs-1}) a = 0.
/// The proof is sound if this equation holds, and complete if a is chosen at random.
/// In practice these points typically should be given as public inputs, and constrained by the decider.
/// base9 - version
pub fn escalarmul_gadget_9<'a, F: PrimeField + FieldUtils, C: CurveExt<Base=F>>(
    circuit: &mut Circuit<'a, F, Gatebb<'a,F>>,
    sc: Variable,
    pt: EcAffinePoint<F,C>,
    num_limbs: usize,
    round: usize,
    a: EcAffinePoint<F,C>,
    b: EcAffinePoint<F,C>,
    nonzeros: &mut Vec<Variable>,
) -> EcAffinePoint<F, C> {
    // The algorithm:
    // We compute a, pt+a, 2pt+a, ..., 8pt+a
    // Then, we start from the last limb, fetch precomputed point using lagrange polynomial
    // Multiply by 9 (using two triplings, which are themselves polynomials of degree 8)
    // Go to the next limb, and so on. Accumulated error becomes a*(1+9+9^2+...9^{num_limbs-1}), which is assumed to be -b.
    // Tripling is checked by verifying 2X = Y-X inside of a polynomial.

    // Compute limbs:

    let limbs = limb_decompose_gadget(circuit, 9, round, num_limbs, sc);
    let mut precomputed_pts = vec![a];
    let mut curr = a;
    for _ in 1..9 {
        curr = ecadd_gadget(circuit, curr, pt, nonzeros, round);
        precomputed_pts.push(curr);
    }

    // Compute lookups:

    let precomputed_pts_prep : Vec<_> = precomputed_pts.iter().map(|pt|vec![pt.x, pt.y]).collect();
    let precomputed_pts_prep : Vec<_> = precomputed_pts_prep.iter().map(|x|x.as_ref()).collect();

    let mut pts_limbs = vec![];
    for i in 0..num_limbs {
        pts_limbs.append(&mut choice_gadget(circuit, &precomputed_pts_prep, limbs[i], round));
    }

    // Compute advices:

    let adv = Advice::new(
        2*num_limbs,
        0,
        8*(num_limbs-1),
        move |args, _| {
            let mut pts = vec![];
            for i in 0..num_limbs {
                pts.push(C::new_jacobian(args[2*i], args[2*i+1], F::ONE).unwrap());
            }
            let mut x3 = vec![];
            let mut x9 = vec![];
            let mut acc = vec![];
            for i in 0..num_limbs {
                if i == 0 {
                    acc.push(pts[num_limbs-i-1])
                } else {
                    acc.push(pts[num_limbs-i-1] + x9[i-1])    
                }

                if i<num_limbs-1 {
                    x3.push(acc[i].double() + acc[i]);
                    x9.push(x3[i].double() + x3[i]);
                }
            }

            let mut zinv : Vec<_> = acc.iter().chain(x3.iter()).chain(x9.iter()).map(|pt|pt.jacobian_coordinates().2).collect();
            
            zinv.batch_invert();

            let mut acc_aff = vec![];
            let mut x3_aff = vec![];
            let mut x9_aff = vec![];

            for i in 0..num_limbs-1 {
                let zacc = zinv[i];
                let zx3 = zinv[i+num_limbs];
                let zx9 = zinv[i+2*num_limbs-1];

                let zacc_sq = zacc.square();
                let zx3_sq = zx3.square();
                let zx9_sq = zx9.square();

                let zacc_cb = zacc*zacc_sq;
                let zx3_cb = zx3*zx3_sq;
                let zx9_cb = zx9*zx9_sq;

                acc_aff.push((acc[i].jacobian_coordinates().0 * zacc_sq, acc[i].jacobian_coordinates().1 * zacc_cb));
                x3_aff.push((x3[i].jacobian_coordinates().0 * zx3_sq, x3[i].jacobian_coordinates().1 * zx3_cb));
                x9_aff.push((x9[i].jacobian_coordinates().0 * zx9_sq, x9[i].jacobian_coordinates().1 * zx9_cb));
            }

            let zacc_last = zinv[num_limbs-1];
            let zacc_last_sq = zacc_last.square();
            let zacc_last_cb = zacc_last*zacc_last_sq;
            acc_aff.push(
                (
                    acc[num_limbs-1].jacobian_coordinates().0 * zacc_last_sq,
                    acc[num_limbs-1].jacobian_coordinates().1 * zacc_last_cb
                )
            );

            let mut scale_factors = vec![]; // Total length 2*(num_limbs-1), for all mul by 3 transitions
            for i in 0..num_limbs-1 {
                scale_factors.push(acc_aff[i].1.scale(2).cube()); // 3rd coordinate of the projective doubling
            }

            for i in 0..num_limbs-1 {
                scale_factors.push(x3_aff[i].1.scale(2).cube());
            }

            scale_factors.batch_invert();


            for i in 0..num_limbs-1 {
                scale_factors[i] *= (acc_aff[i].0 - x3_aff[i].0).cube(); // 3rd coordinate of the projective addition/subtraction
                scale_factors[num_limbs-1+i] *= (x3_aff[i].0 - x9_aff[i].0).cube();
            }


            // scale_factors now contain data we need to compare projective 2A and B-A

            let mut ret = vec![];

            for i in 1..num_limbs{ // Skip the first accumulator as we don't need it.
                ret.push(acc_aff[i].0);
                ret.push(acc_aff[i].1);
            }

            for i in 0..num_limbs-1{
                ret.push(x3_aff[i].0);
                ret.push(x3_aff[i].1);
            }

            for i in 0..num_limbs-1{
                ret.push(x9_aff[i].0);
                ret.push(x9_aff[i].1);
            }

            for i in 0..2*(num_limbs-1){
                ret.push(scale_factors[i]);
            }

            ret // layout: 2(nl-1) accumulators, 2(nl-1) x3, 2(nl-1) x9, 2*(nl-1) scalefactors
    });

    let advices = circuit.advice(
        round,
        adv,
        pts_limbs.clone(),
        vec![]
    );

    let (acc, rest) = advices.split_at(2*(num_limbs-1));
    let (x3, rest) = rest.split_at(2*(num_limbs-1));
    let (x9, rest) = rest.split_at(2*(num_limbs-1));
    let (scale3, scale9) = rest.split_at(num_limbs-1);

    let mut pt_acc = vec![EcAffinePoint::<F,C>::new_unchecked(pts_limbs[2*num_limbs-2], pts_limbs[2*num_limbs-1])];
    // Insert first accumulator back where it belongs.
    let mut pt_x3 = vec![];
    let mut pt_x9 = vec![];

    for i in 0..num_limbs-1 {
        pt_acc.push(EcAffinePoint::<F,C>::new(circuit, acc[2*i], acc[2*i+1]));
        pt_x3.push(EcAffinePoint::<F,C>::new(circuit, x3[2*i], x3[2*i+1]));
        pt_x9.push(EcAffinePoint::<F,C>::new(circuit, x9[2*i], x9[2*i+1]));
    }

    // Check that 2A.rescale(q) = B-A. Notice!! - q must be nonzero. alternative would be using 1/q,
    // but this would increase degree from 8 to 9
    let triple_check = Gatebb::new(
        8,
        5,
        3,
        Rc::new(|args, _|{
            let a = (args[0], args[1]);
            let b = (args[2], args[3]);
            let q = args[4];
            let (x1,y1,z1) = double_proj::<F,C>(a);
            let (x2,y2,z2) = add_proj::<F,C>(b, (args[0], -args[1]));
            vec![x2 - x1*q, y2 - y1*q, z2 - z1*q]
        })
    );

    for i in 0..num_limbs-1 {
        let input = vec![pt_acc[i].x, pt_acc[i].y, pt_x3[i].x, pt_x3[i].y, scale3[i]];
        circuit.constrain(&input, triple_check.clone());
        nonzeros.push(scale3[i]);
        let input = vec![pt_x3[i].x, pt_x3[i].y, pt_x9[i].x, pt_x9[i].y, scale9[i]];
        circuit.constrain(&input, triple_check.clone());
        nonzeros.push(scale9[i]);

        eclin_gadget(circuit,
            pt_x9[i],
            EcAffinePoint::<F,C>::new_unchecked(pts_limbs[2*num_limbs-2-2*(i+1)], pts_limbs[2*num_limbs-1-2*(i+1)]),
            pt_acc[i+1],
            nonzeros,
            round
        )
    }

    let ret = ecadd_gadget(circuit, b, pt_acc[num_limbs-1], nonzeros, round);

    ret


}

// Gadget that checks that 3a = b.
// pub fn tripling_check_gadget<'a, F: PrimeField+FieldUtils, C: CurveExt<Base=F>>(circuit: &mut Circuit<'a, F, Gatebb<'a,F>>, a: EcAffinePoint<F, C>, b: EcAffinePoint<F, C>, round: usize, nonzero_routine: NonzeroSubroutine<'a, F>){
//     let a2 = circuit.apply(
//         round,
//         PolyOp::new(6, 2, 3,
//             Rc::new(|args|{
//                 let (a,b,c) = double_proj::<F,C>((args[0], args[1]));
//                 vec![a,b,c]
//             })
//         ),
//         vec![a.x, a.y]
//     );
//     let b_minus_a = circuit.apply(
//         round,
//         PolyOp::new(8, 4, 3,
//             Rc::new(|args|{
//                 let (a,b,c) = add_proj::<F,C>((args[0], args[1]), (args[2], -args[3]));
//                 vec![a,b,c]
//             })
//         ),
//         vec![b.x, b.y, a.x, a.y]
//     );
// }