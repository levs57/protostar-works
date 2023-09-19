use std::{iter::repeat, cmp::max, marker::PhantomData, rc::Rc};

use group::{Group, Curve};
use ff::{Field, PrimeField};
use halo2::{arithmetic::{best_multiexp, best_fft}};
use halo2curves::CurveAffine;
use num_traits::pow;
use rand_core::OsRng;

pub trait RootsOfUnity where Self : PrimeField{
    /// Returns power of a primitive root of unity of order 2^logorder.
    fn roots_of_unity(power: u64, logorder: usize) -> Self;
    /// Returns power of 1/2.
    fn half_pow(power: u64) -> Self;
    /// Returns FFT of the binomial.
    fn binomial_FFT(power: usize, logorder: usize) -> Vec<Self>;
}

pub fn check_poly<'a, F: PrimeField>(d: usize, i: usize, o:usize, f: Rc<dyn Fn(&[F]) -> Vec<F> + 'a>){
    let a : Vec<_> = repeat(F::random(OsRng)).take(i).collect(); 
    let b : Vec<_> = repeat(F::random(OsRng)).take(i).collect(); 
    
    let mut lcs : Vec<Vec<_>> = vec![];

    let n = d+1; // we need to compute a+tb in d+2 different points

    for j in 0..n+1 {
        let tmp : Vec<F> = a.iter().zip(b.iter()).map(|(a,b)|*a+F::from(j as u64)*b).collect();
        lcs.push((*f)(&tmp));
    }

    assert!(lcs[0].len() == o);

    let mut acc = vec![F::ZERO; i];
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

    for val in acc {
        assert!(val == F::ZERO, "Provided polynomial is not of degree d");
    }
}

#[derive(Clone)]
/// A generic black-box gate. This API is unsafe, you must guarantee that given value is a
/// polynomial of degree d with i inputs and o outputs.
pub struct Gatebb<'a, F : PrimeField> {
    d : usize,
    i : usize,
    o : usize,
    f : Rc<dyn Fn(&[F]) -> Vec<F> + 'a>,
}


impl<'a, F: PrimeField> Gatebb<'a, F> {
    pub fn new(d: usize, i: usize, o: usize, f: Rc<dyn Fn(&[F]) -> Vec<F> + 'a>) -> Self {
        check_poly(d, i, o, f.clone());
        Gatebb::<'a>{d,i,o,f}
    } 
    pub fn new_unchecked(d: usize, i: usize, o: usize, f: Rc<dyn Fn(&[F]) -> Vec<F> + 'a>) -> Self {
        Gatebb::<'a>{d,i,o,f}
    }

}
pub trait Gate<F : PrimeField> : Clone {
    /// Returns degree.
    fn d(&self) -> usize;
    /// Returns input size.
    fn i(&self) -> usize;
    /// Returns output size.
    fn o(&self) -> usize;
    /// Executes gate on a given input.
    fn exec(& self, input : &[F]) -> Vec<F>;
}

impl<'a, F : PrimeField + RootsOfUnity> Gate<F> for Gatebb<'a, F> {
    /// Returns degree.
    fn d(&self) -> usize {
        self.d
    }
    /// Returns input size.
    fn i(&self) -> usize{
        self.i
    }
    /// Returns output size.
    fn o(&self) -> usize{
        self.o
    }
    /// Executes gate on a given input.
    fn exec(&self, input : &[F]) -> Vec<F>{
        (self.f)(input)
    }


    // fn cross_terms_adjust(&self, in1: &Vec<F>, in2: &Vec<F>, deg: usize) -> Vec<Vec<F>> {
    //     assert!(self.d() <= deg, "Can not adjust downwards.");
        
    //     let mut d = deg;
    //     if d == 0 {
    //         return vec![self.exec(in1)]
    //     }
        
    //     let mut logorder = 0;
    //     while d>0 {
    //         d>>=1;
    //         logorder +=1;
    //     }

    //     let mut values = vec![vec![]; self.o];

    //     let omega_inv = F::roots_of_unity(pow(2, logorder)-1, logorder);
    //     let scale = F::half_pow(logorder as u64);

    //     let binomial = F::binomial_FFT(deg-self.d(), logorder);

    //     for i in 0..pow(2, logorder){
    //         let t = F::roots_of_unity(i, logorder);
    //         let fgsds : Vec<_> = in1.iter().zip(in2.iter()).map(|(x,y)| (*x + *y * t)).collect();
    //         let tmp = self.exec(&fgsds);
    //         for j in 0..self.o() {
    //             values[j].push(tmp[j]);
    //         }
    //     }

    //     if deg>self.d(){
    //         for i in 0..pow(2, logorder){
    //             for j in 0..self.o() {
    //                 values[j][i] *= binomial[i];
    //             }
    //         }
    //     }

    //     values.iter_mut().map(|v| {
    //         best_fft(v, omega_inv, logorder as u32);
    //         v.iter_mut().map(|x|*x *= scale).count();
    //     }).count();

    //     let mut ret = vec![vec![]; (deg+1)];
    //     for i in 0..(deg+1) {
    //         for j in 0..self.o {
    //             ret[i].push(values[j][i])
    //         }
    //     }

    //     assert!({
    //         let mut flag = true;
    //         for i in (deg+1)..pow(2,logorder) {
    //             for j in 0..self.o{
    //                 flag &= (values[j][i] == F::ZERO)
    //             }
    //         }
    //         flag
    //     }, "fft failed");

    //     ret

    // }

}