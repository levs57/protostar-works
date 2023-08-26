use ff::PrimeField;
use halo2curves::CurveAffine;

use crate::{gate::{self, Gate}, constraint_system::{self, ConstraintSystem}, commitment::Ck};

pub struct Circuit<'a, F : PrimeField, G : CurveAffine<Base=F>, CK: Ck<G>> {
    cs : ConstraintSystem<'a, F>,
    pub_values : Vec<Option<F>>,
    round_values : Vec<Vec<Option<F>>>,
    challenges : Vec<Vec<Option<F>>>,
    exec : Vec<Box<dyn 'a + FnMut(&mut Self) -> ()>>,
    ck : CK,
}



pub fn apply<'a, F:PrimeField>(cs: &mut ConstraintSystem<'a, F>, gate: Box<dyn 'a + Gate<'a, F>>) {

}