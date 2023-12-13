use std::marker::PhantomData;

use ff::PrimeField;

use crate::{circuit::{CircuitRun, PolyOp, ConstructedCircuit}, gate::Gate, witness::{ProtostarWtns, CSWtns}, constraint_system::ProtoGalaxyConstraintSystem};

pub trait ForeignCircuitInstance {
    type Inputs;
    type Outputs;
    type Witness;

    /// Sets circuit inputs for the next execution
    fn set_inputs(&mut self, inputs: &Self::Inputs) -> ();

    /// Executes circuit until first missing external value
    fn execute(&mut self) -> Self::Outputs;

    /// Computes protostar challenges and outputs completed witness
    fn terminate(self) -> Self::Witness;
}

//
// x.set_inputs
// x.execute_round
// x.set_inputs
// x.execute_round
// x.set_inputs
// x.execute_round
// x.set_inputs
// x.execute_round
// x.set_inputs
// x.execute_round
// x.set_inputs
// x.execute_round
// x.terminate
//

// (a, b, s, c=a+sb, C=commiit(run))

pub trait ForeignCircuit {
    type Instance: ForeignCircuitInstance;
    type ConstraintSystem;

    fn spawn(&self) -> Self::Instance;

    fn get_cs(&self) -> &Self::ConstraintSystem;
}











struct ForeignCircuitComponent<MainCircuit, CInputs, COutputs, Advisor, FCInputs, FCInstance, FC>
where 
    FCInstance: ForeignCircuitInstance<Inputs = FCInputs>,
    FC: ForeignCircuit<Instance = FCInstance>,
    Advisor: Fn(&mut MainCircuit, CInputs) -> (COutputs, FCInputs),
{
    foreign_circuit: FC,
    advicer: Advisor,

    _phantom_data: PhantomData<(MainCircuit, CInputs, COutputs)>
}

impl<MainCircuit, CInputs, COutputs, Advisor, FCInputs, FCInstance, FC> 
ForeignCircuitComponent<MainCircuit, CInputs, COutputs, Advisor, FCInputs, FCInstance, FC>
where 
    FCInstance: ForeignCircuitInstance<Inputs = FCInputs>,
    FC: ForeignCircuit<Instance = FCInstance>,
    Advisor: Fn(&mut MainCircuit, CInputs) -> (COutputs, FCInputs),
{
    pub fn new(foreign_circuit: FC, advicer: Advisor) -> Self {
        Self { 
            foreign_circuit,
            advicer,
            _phantom_data: PhantomData,
        }
    }

    fn _apply(&mut self, inputs: &FCInputs) {
        let mut spawned = self.foreign_circuit.spawn();
        spawned.set_inputs(inputs);
        spawned.execute();
        spawned.terminate();
        todo!()
    }

    fn apply(&mut self, main_circuit: &mut MainCircuit, inputs: CInputs) -> COutputs {
        let (coutputs, fcinputs) = (self.advicer)(main_circuit, inputs);
        self._apply(&fcinputs);
        coutputs
    }
}