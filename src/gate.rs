use std::fmt::Debug;
use std::rc::Rc;

use ff::PrimeField;

use crate::utils::poly_utils::check_poly;
use crate::utils::field_precomp::FieldUtils;


/// A generic black-box gate. This API is unsafe, you must guarantee that given value is a
/// polynomial of degree d with i inputs and o outputs.
#[derive(Clone)]
pub struct Gatebb<'a, F : PrimeField> {
    d : usize,
    i : usize,
    o : usize,
    f : Rc<dyn Fn(&[F], &[F]) -> Vec<F> + 'a>,
    consts: Vec<F>,
}

impl<'a, F: PrimeField> Debug for Gatebb<'a, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gatebb").field("d", &self.d).field("i", &self.i).field("o", &self.o).field("f", &"<anonymous>").finish()
    }
}

impl<'a, F: PrimeField> Gatebb<'a, F> {
    pub fn new(d: usize, i: usize, o: usize, f: Rc<dyn Fn(&[F], &[F]) -> Vec<F> + 'a>, consts: Vec<F>) -> Self {
        check_poly(d, i, o, f.clone(), &consts).unwrap();
        Gatebb::<'a>{d, i, o, f, consts}
    }
    pub fn new_unchecked(d: usize, i: usize, o: usize, f: Rc<dyn Fn(&[F], &[F]) -> Vec<F> + 'a>, consts: Vec<F>) -> Self {
        Gatebb::<'a>{d, i, o, f, consts}
    }

}
pub trait Gate<'a, F : PrimeField> : Clone + Debug {
    /// Returns degree.
    fn d(&self) -> usize;
    /// Returns input size.
    fn i(&self) -> usize;
    /// Returns output size.
    fn o(&self) -> usize;
    /// Executes gate on a given input.
    fn exec(& self, input : &[F]) -> Vec<F>;
}

impl<'a, F : PrimeField + FieldUtils> Gate<'a, F> for Gatebb<'a, F> {
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
        (self.f)(input, &self.consts)
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