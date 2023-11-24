use std::marker::PhantomData;

use ff::PrimeField;
use group::{prime::PrimeCurveAffine, Group, Curve};
use halo2::halo2curves::{CurveExt, CurveAffine};

use crate::{circuit::{ConstructedCircuit, Circuit, ExternalValue, CircuitRun}, gate::Gatebb, utils::{field_precomp::FieldUtils, arith_helper::j2a}, gadgets::{arith::read_const_gadget, ecmul::EcAffinePoint, input::input}, folding::encode::Encoded, constraint_system::Variable, external_interface::InnerValue};
use super::{ecmul::{escalarmul_gadget_9, eclin_gadget}, lookup::StaticLookup, nonzero_check::Nonzeros};


// Convention: Cp - primary curve, i.e. bn254, Cs - secondary curve (which is primary for cyclefold)
// Fp = Cp::Scalar, Fs = Cs :: Scalar

pub struct ConstructedCyclefoldCircuit<
    'circuit,
    Fs: PrimeField+FieldUtils,
    Fp: PrimeField+FieldUtils,
    C: CurveExt<ScalarExt=Fp, Base=Fs>,
>
{
    pub constructed: ConstructedCircuit<'circuit, Fs, Gatebb<'circuit, Fs>>,
    pub pt_acc : (ExternalValue<Fs>, ExternalValue<Fs>),
    pub pt_inc : (ExternalValue<Fs>, ExternalValue<Fs>),
    pub pt_res : (ExternalValue<Fs>, ExternalValue<Fs>),
    pub sc : ExternalValue<Fs>,

    _marker: PhantomData<(Fp, C)>
}


/// Constructs a circuit checking that PT_ACC + SC * PT_INC = PT_RES.
/// Might be incomplete for some offset points, so ensure that offset is generated randomly
/// in multi-prover case, this is a DoS vector - need to ensure that offset is chosen after
/// all the data is already committed.
/// Also, it will fail in case of the collision - i.e. if PT_ACC != SC*PT_INC != PT_RES condition
/// breaks. Avoid this (this never happens if scalar is chosen randomly). 
pub fn construct_cyclefold_circuit<
    'circuit,
    Fs: PrimeField+FieldUtils,
    Fp: PrimeField+FieldUtils,
    C: CurveExt<ScalarExt=Fp, Base=Fs>
> (
    offset_point: C,
) -> ConstructedCyclefoldCircuit<'circuit, Fs, Fp, C>{
    let mut circuit = Circuit::<Fs, Gatebb<'circuit,Fs>>::new(10, 1);
    let num_limbs = 41;
    let a = offset_point;
    let scale = (Fp::from(9).pow([num_limbs as u64])-Fp::ONE)*(Fp::from(8).invert().unwrap());
    let b = a*scale;

    let a = j2a(a.jacobian_coordinates());
    let b = j2a(b.jacobian_coordinates());

    let var_a = (
        read_const_gadget(&mut circuit, a.0, 0),
        read_const_gadget(&mut circuit, a.1, 0)
    );

    let var_b = (
        read_const_gadget(&mut circuit, b.0, 0),
        read_const_gadget(&mut circuit, b.1, 0)
    );

    let pt_a = EcAffinePoint::<Fs, C>::new_unchecked(var_a.0, var_a.1);
    let pt_b = EcAffinePoint::new_unchecked(var_b.0, var_b.1);

    let mut nonzeros = Nonzeros::new(9);

    let scalar_inp = circuit.ext_val(1)[0];
    let sc = input(&mut circuit, scalar_inp, 0);

    let pt_inc = circuit.ext_val(2);
    let pt_inc = (pt_inc[0], pt_inc[1]);
    let pt_inc_x = input(&mut circuit, pt_inc.0, 0);
    let pt_inc_y = input(&mut circuit, pt_inc.1, 0);

    let incoming_point = EcAffinePoint::new(&mut circuit, pt_inc_x, pt_inc_y);

    let pt_acc = circuit.ext_val(2);
    let pt_acc = (pt_acc[0], pt_acc[1]);
    let pt_acc_x = input(&mut circuit, pt_acc.0, 0);
    let pt_acc_y = input(&mut circuit, pt_acc.1, 0);

    let accumulated_point = EcAffinePoint::new(&mut circuit, pt_acc_x, pt_acc_y);

    let pt_res = circuit.ext_val(2);
    let pt_res = (pt_res[0], pt_res[1]);

    let pt_res_x = input(&mut circuit, pt_res.0, 0);
    let pt_res_y = input(&mut circuit, pt_res.1, 0);

    let result_point = EcAffinePoint::new(&mut circuit, pt_res_x, pt_res_y);

    let prod = escalarmul_gadget_9 (
        &mut circuit,
        sc,
        incoming_point,
        num_limbs,
        0,
        pt_a,
        pt_b,
        &mut nonzeros
    );
    eclin_gadget(&mut circuit, prod, accumulated_point, result_point, &mut nonzeros, 0);

    nonzeros.finalize(&mut circuit);
    let constructed = circuit.finalize();
    ConstructedCyclefoldCircuit { constructed, pt_acc, pt_inc, pt_res, sc : scalar_inp, _marker: PhantomData::<(Fp, C)> }
}


// pub struct CyclefoldWitness<Fs : PrimeField+FieldUtils, Fp : PrimeField+FieldUtils> {
//     pub witness: Vec<Fs>, // should be protostar witness
//     pub e: Fs,
//     pub pt_acc: (Fs, Fs), // these are ec points in affine form
//     pub pt_inc: (Fs, Fs),
//     pub pt_res: (Fs, Fs),
//     pub sc : Fs,
// }




/// This component can consume non-native elliptic curve ops. 
pub struct CyclefoldComponent<
    'cfold,
    F: PrimeField+FieldUtils,
    F2: PrimeField+FieldUtils,
    C: CurveExt<ScalarExt=F2, Base=F>
> {
    constructed_cfold: ConstructedCyclefoldCircuit<'cfold, F, F2, C>,

}