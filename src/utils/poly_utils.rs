use std::rc::Rc;

use ff::PrimeField;
use rand_core::OsRng;

/// Little endian bit decomposition of n
pub fn bits_le(n: usize) -> Vec<u64> {
    let mut n = n;
    let mut bits = vec![];

    while n > 0 {
        bits.push((n as u64) & 1);
        n >>= 1;
    }

    bits
}

pub fn check_poly<'a, F: PrimeField>(d: usize, i: usize, o:usize, f: Rc<dyn Fn(&[F], &[F]) -> Vec<F> + 'a>) -> Result<(), &str>{
    let mut a = vec![]; for _ in 0..i {a.push(F::random(OsRng))} 
    let mut b = vec![]; for _ in 0..i {b.push(F::random(OsRng))} 

    let mut lcs : Vec<Vec<_>> = vec![];

    let n = d+1; // we need to compute a+tb in d+2 different points

    for j in 0..n+1 {
        let tmp : Vec<F> = a.iter().zip(b.iter()).map(|(a,b)|*a+F::from(j as u64)*b).collect();
        lcs.push((*f)(&tmp, &[]));
    }

    assert!(lcs[0].len() == o);

    let mut acc = vec![F::ZERO; o];
    let mut binom = F::ONE;

    // n!/k!(n-k)! = n!/(k-1)!(n-k+1)! * (n-k+1)/k
    for k in 0..n+1 {
        if k>0 {
            binom *= F::from((n-k+1) as u64) * F::from(k as u64).invert().unwrap() 
        }
        let sign = F::ONE-F::from(((k%2)*2) as u64);
        acc.iter_mut().zip(lcs[k].iter()).map(|(acc, upd)|{
            *acc += sign * (*upd) * binom;
        }).count();
    }

    let mut flag = true;

    for val in acc {
        flag = flag & (val == F::ZERO);
    }

    match flag {
        true => Ok(()),
        false => Err("The provided polynomial has degree larger than d"),
    }
}

/// Attempts to find a polynomial degree of a black-box function. Should instead use binary search, of course :).
pub fn find_degree<'a, F: PrimeField>(max_degree: usize, i: usize, o:usize, f: Rc<dyn Fn(&[F], &[F]) -> Vec<F> + 'a>) -> Result<usize, &str>{
    let mut top = 1;
    loop {
        if top > max_degree {return Err("The degree of provided function is too large.")}
        match check_poly(top, i, o, f.clone()) {
            Err(_) => top*=2,
            Ok(()) => break, 
        }
    }
    let mut bot = top/2;
    if bot == 0 {return Ok(1)}
    while top-bot > 1 {
        let mid = (top+bot)/2;
        match check_poly(mid, i, o, f.clone()) {
            Err(_) => bot = mid,
            Ok(()) => top = mid,
        }
    }

    Ok(top)
}