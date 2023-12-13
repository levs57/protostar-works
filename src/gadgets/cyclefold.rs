use std::marker::PhantomData;

use ff::PrimeField;
use group::{prime::PrimeCurveAffine, Group, Curve};
use halo2::halo2curves::{CurveExt, CurveAffine};

use crate::{circuit::{ConstructedCircuit, Circuit, ExternalValue, CircuitRun}, gate::Gatebb, utils::{field_precomp::FieldUtils, arith_helper::j2a}, gadgets::{arith::read_const_gadget, ecmul::EcAffinePoint, input::input}, folding::encode::Encoded, constraint_system::Variable, external_interface::InnerValue, witness::ProtostarWtns};
use super::{ecmul::{escalarmul_gadget_9, eclin_gadget}, lookup::StaticLookup, nonzero_check::Nonzeros, rangecheck_common::VarRange};


// Convention: Cp - primary curve, i.e. bn254, Cs - secondary curve (which is primary for cyclefold)
// Fp = Cp::Scalar, Fs = Cs :: Scalar

pub struct ConstructedCyclefoldCircuit<
    'circuit,
    Fs: PrimeField+FieldUtils,
    Fp: PrimeField+FieldUtils,
    Cp: CurveExt<ScalarExt=Fp, Base=Fs>,
>
{
    pub constructed: ConstructedCircuit<'circuit, Fs, Gatebb<'circuit, Fs>>,
    pub pt_acc : (ExternalValue<Fs>, ExternalValue<Fs>),
    pub pt_inc : (ExternalValue<Fs>, ExternalValue<Fs>),
    pub pt_res : (ExternalValue<Fs>, ExternalValue<Fs>),
    pub sc : ExternalValue<Fs>,
    _marker: PhantomData<Cp>
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
    Cp: CurveExt<ScalarExt=Fp, Base=Fs>,
> (
    offset_point: Cp,
) -> ConstructedCyclefoldCircuit<'circuit, Fs, Fp, Cp> {
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

    let pt_a = EcAffinePoint::<Fs, Cp>::new_unchecked(var_a.0, var_a.1);
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
    ConstructedCyclefoldCircuit { constructed, pt_acc, pt_inc, pt_res, sc : scalar_inp, _marker: PhantomData }
}


// pub struct CyclefoldWitness<Fs : PrimeField+FieldUtils, Fp : PrimeField+FieldUtils> {
//     pub witness: Vec<Fs>, // should be protostar witness
//     pub e: Fs,
//     pub pt_acc: (Fs, Fs), // these are ec points in affine form
//     pub pt_inc: (Fs, Fs),
//     pub pt_res: (Fs, Fs),
//     pub sc : Fs,
// }


pub struct CyclefoldInstanceExternalView <
    Fs: PrimeField+FieldUtils,
    Fp: PrimeField+FieldUtils,
    Cp: CurveExt<ScalarExt=Fp, Base=Fs>
> {
    acc_limbs: [[VarRange<Fp>; 3]; 2],
    inc_limbs: [[VarRange<Fp>; 3]; 2],
    ret_limbs: [[VarRange<Fp>; 3]; 2],
    sc_limbs: [VarRange<Fp>; 3],
    error_limbs: [VarRange<Fp>; 3],
    protostar_challenges: Vec<[VarRange<Fp>; 3]>,
    exec_trace_commitment: EcAffinePoint<Fs, Cp>,
    
    _marker: PhantomData<Fs>
}

impl<
    Fs:PrimeField+FieldUtils,
    Fp: PrimeField+FieldUtils,
    Cp: CurveExt<Base = Fs, ScalarExt = Fp>,
> CyclefoldInstanceExternalView<Fs, Fp, Cp>
{

}

/// This component can consume non-native elliptic curve ops. 
pub struct CyclefoldComponent<
    'constructed,
    'cfold,
    Fs: PrimeField+FieldUtils,
    Fp: PrimeField+FieldUtils,
    Cp: CurveExt<ScalarExt=Fp, Base=Fs>
> {
    constructed_cfold: &'constructed ConstructedCyclefoldCircuit<'cfold, Fs, Fp, Cp>,
    
    accumulated_cfold_witness: InnerValue<ProtostarWtns<Fs>>,
    incoming_cfold_witness: InnerValue<ProtostarWtns<Fs>>,

    accumulated_cfold_instance: CyclefoldInstanceExternalView<Fs, Fp, Cp>,

}

impl<
    'constructed,
    'cfold,
    Fs: PrimeField+FieldUtils,
    Fp: PrimeField+FieldUtils,
    Cp: CurveExt<ScalarExt=Fp, Base=Fs>
> CyclefoldComponent<'constructed, 'cfold, Fs, Fp, Cp> {
    pub fn new<'circuit>(
        circuit: &'circuit mut Circuit<'circuit, Fp, Gatebb<'circuit, Fp>>,
        ccc: &'constructed ConstructedCyclefoldCircuit<'cfold, Fs, Fp, Cp>,
        acw: Option<ProtostarWtns<Fs>>,
    ) -> Self
    {
        todo!()
    }

    // pub fn delegate_ec_op<'circuit> (
    //     &mut self,
    //     circuit: &'circuit mut Circuit<'circuit, Fp, Gatebb<'circuit, Fp>>,
    //     a: [[VarRange<Fp>; 3]; 2],
    //     b: [[VarRange<Fp>; 3]; 2],
    //     s: VarRange<Fp>
        
    // )
}
