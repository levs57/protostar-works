use std::{cmp::max, iter::repeat, rc::Rc};

use ff::{PrimeField, Field};
use group::{Group, Curve};
use halo2::arithmetic::best_fft;
use halo2curves::{bn256, grumpkin, CurveAffine, CurveExt};
use num_traits::pow;
use rand_core::{OsRng, RngCore};

use crate::{gate::{find_degree, RootsOfUnity, check_poly}, gadgets::ecmul::{oct_naive, hex_naive, best_mul_proj, mul_doubling_phase}};


type F = bn256::Fr;
type C = grumpkin::G1;


/// One-dimensional gcd
pub fn gcd<F: PrimeField>(a: &[F], b: &[F]) -> Vec<F> {

    let mut poly_a : Vec<F> = a.to_vec();
    let mut poly_b : Vec<F> = b.to_vec();


    let mut a = &mut poly_a;
    let mut b = &mut poly_b;

    while match a.last() {Some(x) => *x == F::ZERO, None => {return poly_b}} {a.pop();}
    while match b.last() {Some(x) => *x == F::ZERO, None => {return poly_a}} {b.pop();}

    while b.len() > 0 && a.len()>0 {
        if a.len() < b.len() {(a,b)=(b,a)}
        let scale = (*b).last().unwrap().invert().unwrap()*(*a).last().unwrap();
        for j in 0..b.len() {
            let la = a.len(); let lb = b.len();
            (*a)[la-j-1] -= (*b)[lb-j-1]*scale;
        }
        while match (*a).last() {Some(x) => *x == F::ZERO, None => false} {a.pop();}        
    }
    
    if poly_a.len() == 0 {poly_b} else {poly_a}
}

/// Returns (degree, gcd degree).
pub fn gcd_mulvar_deg<'a, F: PrimeField + RootsOfUnity>(max_degree: usize, i: usize, o:usize, f: Rc<dyn Fn(&[F]) -> Vec<F> + 'a>) -> (usize, usize) {
    let deg = find_degree(max_degree, i, o, f.clone()).unwrap();

    let mut v = vec![]; for _ in 0..i {v.push(F::random(OsRng))}
    let mut w = vec![]; for _ in 0..i {w.push(F::random(OsRng))}

    let mut degbin = 1;
    let mut logorder = 0;
    while degbin < (deg+1) {degbin <<= 1; logorder+=1;}

    let mut values = vec![vec![]; o];
    
    for k in 0..pow(2, logorder){
        let t = F::roots_of_unity(k, logorder);
        let fgsds : Vec<_> = v.iter().zip(w.iter()).map(|(x,y)| (*x + *y * t)).collect();
        let tmp = f(&fgsds);
        for j in 0..o {
            values[j].push(tmp[j]);
        }

    }
    let omega_inv = F::roots_of_unity(pow(2, logorder)-1, logorder);
    let scale = F::half_pow(logorder as u64);

    values.iter_mut().map(|v| {
        best_fft(v, omega_inv, logorder as u32);
        v.iter_mut().map(|x|*x *= scale).count();
    }).count();
    
    (deg, values.iter().fold(vec![], |acc, upd| gcd(&acc, &upd)).len()-1)
}
#[test]
fn test_gcd(){
    let a = vec![F::from(3), F::from(5), F::from(2)];
    let b = vec![F::from(1),];
    assert!(gcd(&a,&b).len() == 1);
}

#[test]
fn test_gcd_mulvar(){

    let mut ret1 = vec![];
    let mut ret2 = vec![];

    for k in 2..20{
        let pt = C::random(OsRng);
        let s = F::random(OsRng);
        
        let test_input = (pt.x*s, pt.y*s, s);
        let test_output = mul_doubling_phase::<F,C>(test_input, k);
        
        let s2 = test_output.2.invert().unwrap();
        
        assert!(pt*<C as CurveExt>::ScalarExt::from(k) == grumpkin::G1Affine::from_xy(test_output.0*s2, test_output.1*s2).unwrap().into());

        //println!("Processing map A->{}A", k);
        let tmp = gcd_mulvar_deg::<F>(17000, 3, 3, Rc::new(move |args|{
            let tmp = mul_doubling_phase::<F,C>((args[0], args[1], args[2]), k);
            assert!(args[1] != args[0]);
            vec![tmp.0, tmp.1, tmp.2]
        }));

        ret1.push(tmp.0);
        ret2.push(tmp.0 - tmp.1);
    }

    println!("-------------------------------------------------");
    println!("{:?}", (2..20).collect::<Vec<_>>());
    println!("{:?}", ret1);
    println!("{:?}", ret2);
}

//w = 3x^2, s = yz, b=x y^2 z
//h = 9x^4 - 8 x y^2 z = x (9 x^3 - 8 y^2 z)
// x' = 2 xy (9x^3 - 8 y^2)
// y' = 3x^2*(12 x y^2  - 9x^4) - 8 y^4 
// z' = 8 y^3