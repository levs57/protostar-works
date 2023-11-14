use std::rc::Rc;
use elsa::map::FrozenMap;
use ff::PrimeField;
use crate::gate::Gatebb;
use crate::utils::field_precomp::FieldUtils;
use gate_macro::make_gate;

#[make_gate]
fn nonzero_check<'c, F: PrimeField + FieldUtils>() -> Gatebb<'c, F> {
    Gatebb::new(2, 2, 1, Rc::new(|args|vec![args[0]*args[1] - F::ONE]))
}