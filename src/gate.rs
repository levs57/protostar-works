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

#[derive(Clone)]
/// A generic black-box gate. This API is unsafe, you must guarantee that given value is a
/// homogeneous polynomial of degree d with i inputs and o outputs. It will do a sanity check
/// so if a polynomial of different degree, or non-homogeneous one is provided, it will fail. 
pub struct Gatebb<'a, F : PrimeField> {
    d : usize,
    i : usize,
    o : usize,
    f : Rc<dyn Fn(&[F]) -> Vec<F> + 'a>,
}

impl<'a, F: PrimeField> Gatebb<'a, F> {
    pub fn new(d: usize, i: usize, o: usize, f: Rc<dyn Fn(&[F]) -> Vec<F> + 'a>) -> Self {
        let random_input : Vec<_> = repeat(F::random(OsRng)).take(i).collect(); 
        let random_input_2 : Vec<_> = random_input.iter().map(|x| *x*F::from(2)).collect();
        assert!({
            let mut flag = true;
            (&f)(&random_input_2).iter().zip((&f)(&random_input).iter()).map(|(a, b)| flag &= (*a==*b*F::from(pow(2, d)))).count();
            flag
        }, "Sanity check failed - provided f is not a polynomial of degree d");
        Gatebb::<'a>{d,i,o,f}
    } 
    pub fn new_unchecked(d: usize, i: usize, o: usize, f: Rc<dyn Fn(&[F]) -> Vec<F> + 'a>) -> Self {
        Gatebb::<'a>{d,i,o,f}
    }

    /// Converts a nonuniform polynomial to a uniform one.
    /// Will not work for relaxation factor = 0, however this never occurs in folding schemes.
    /// Increases i by 1 - first argument is a relaxation factor.
    pub fn from_nonuniform<'b : 'a>(d: &'a usize, i: &'a usize, o: &'a usize, f: &'b Rc<dyn Fn(&[F]) -> Vec<F> + 'a>) -> Self {
        let g = |args: &[F]|{
            let t_inv = args[0].invert().unwrap();
            let mut args_internal = vec![];
            for s in 0..*i {
                args_internal.push(t_inv * args[s+1])
            }
            f(&args_internal).iter().map(|x|*x*t_inv.pow([*d as u64])).collect()
        };

        Self::new(*d, *i+1, *o, Rc::new(g))
    }

}

#[derive(Clone)]
pub struct AdjustedGate<F: PrimeField, T: Gate<F> + Sized> {
    gate: T,
    deg: usize,
    _marker: PhantomData<F>,
}

pub trait Gate<F : PrimeField> : Clone {
    /// Returns degree.
    fn d(&self) -> usize;
    /// Returns input size.
    fn i(&self) -> usize;
    /// Returns output size.
    fn o(&self) -> usize;
    /// Executes gate on a given input. Must ensure the correct length of an input.
    fn exec(& self, input : &[F]) -> Vec<F>;
    /// Returns coefficients of  f(in1 + x in2) in x (for example, 0-th is f(in1) and d-th is f(in2))
    fn cross_terms(&self, in1: &Vec<F>, in2: &Vec<F>) -> Vec<Vec<F>>;
    /// Computes cross-terms for the higher degree by using symbolic multiplication by binomial.
    fn cross_terms_adjust(&self, in1: &Vec<F>, in2: &Vec<F>, deg: usize) -> Vec<Vec<F>>;
    /// Returns adjusted gate.
    fn adjust(self, deg: usize) -> AdjustedGate<F, Self> where Self: Sized;
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
    /// Executes gate on a given input. Must ensure the correct length of an input.
    fn exec(&self, input : &[F]) -> Vec<F>{
        assert!(input.len() == self.i);
        let tmp = (self.f)(input);
        assert!(tmp.len() == self.o);
        tmp
    }

    /// Returns coefficients of  f(in1 + x in2) in x (for example, 0-th is f(in1) and d-th is f(in2))
    fn cross_terms(&self, in1: &Vec<F>, in2: &Vec<F>) -> Vec<Vec<F>> {
        self.cross_terms_adjust(in1, in2, self.d())
    }    
    
    fn cross_terms_adjust(&self, in1: &Vec<F>, in2: &Vec<F>, deg: usize) -> Vec<Vec<F>> {
        assert!(self.d() <= deg, "Can not adjust downwards.");
        
        let mut d = deg;
        if d == 0 {
            return vec![self.exec(in1)]
        }
        
        let mut logorder = 0;
        while d>0 {
            d>>=1;
            logorder +=1;
        }

        let mut values = vec![vec![]; self.o];

        let omega_inv = F::roots_of_unity(pow(2, logorder)-1, logorder);
        let scale = F::half_pow(logorder as u64);

        let binomial = F::binomial_FFT(deg-self.d(), logorder);

        for i in 0..pow(2, logorder){
            let t = F::roots_of_unity(i, logorder);
            let fgsds : Vec<_> = in1.iter().zip(in2.iter()).map(|(x,y)| (*x + *y * t)).collect();
            let tmp = self.exec(&fgsds);
            for j in 0..self.o() {
                values[j].push(tmp[j]);
            }
        }

        if deg>self.d(){
            for i in 0..pow(2, logorder){
                for j in 0..self.o() {
                    values[j][i] *= binomial[i];
                }
            }
        }

        values.iter_mut().map(|v| {
            best_fft(v, omega_inv, logorder as u32);
            v.iter_mut().map(|x|*x *= scale).count();
        }).count();

        let mut ret = vec![vec![]; (deg+1)];
        for i in 0..(deg+1) {
            for j in 0..self.o {
                ret[i].push(values[j][i])
            }
        }

        assert!({
            let mut flag = true;
            for i in (deg+1)..pow(2,logorder) {
                for j in 0..self.o{
                    flag &= (values[j][i] == F::ZERO)
                }
            }
            flag
        }, "fft failed");

        ret

    }

    fn adjust(self, deg: usize) -> AdjustedGate<F, Self> where Self: Sized {
        assert!(deg >= self.d(), "Can only adjust upwards");
        AdjustedGate { gate : self, deg, _marker : PhantomData }
    }
}

impl<F: PrimeField, T: Gate<F>> Gate<F> for AdjustedGate<F, T>{
    fn d(&self) -> usize {
        self.deg
    }

    fn i(&self) -> usize {
        self.gate.i()
    }

    fn o(&self) -> usize {
        self.gate.o()
    }

    fn exec(& self, input : &[F]) -> Vec<F> {
        self.gate.exec(input)
    }

    fn cross_terms(&self, in1: &Vec<F>, in2: &Vec<F>) -> Vec<Vec<F>> {
        self.gate.cross_terms_adjust(in1, in2, self.deg)
    }

    fn cross_terms_adjust(&self, in1: &Vec<F>, in2: &Vec<F>, deg: usize) -> Vec<Vec<F>> {
        panic!("Should not be called on adjusted gate")
    }

    fn adjust(self, deg: usize) -> AdjustedGate<F, Self> where Self: Sized {
        assert!(deg >= self.d(), "Can only adjust upwards");
        AdjustedGate { gate : self, deg, _marker : PhantomData }
    }
}