use std::{marker::PhantomData, rc::Rc};

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



pub type VarIdx = usize;

#[derive(Clone)]
pub struct Var<T> {
    addr: VarIdx,
    _marker: PhantomData<T>,
}

pub trait TStorage {
    fn get<T: 'static>(&self, addr: Var<T>) -> Option<Rc<T>>;
    fn set<T: 'static >(&mut self, value: T, addr: Var<T>) -> Result<(), String>;
    fn replace<T: 'static >(&mut self, value: T, addr: Var<T>) -> ();
}

pub trait TStorageBuilder {
    type Instance: TStorage;

    fn alloc<T: 'static>(&mut self) -> Var<T>;

    fn instanciate(self) -> Self::Instance;
}

pub trait TOperation<Storage: TStorage> {
    fn inputs(&self) -> &[VarIdx];
    fn outputs(&self) -> &[VarIdx];
    fn execute(self, storage: &mut Storage);
}

pub struct Operation<Storage: TStorage> {
    inputs: Vec<VarIdx>,
    outputs: Vec<VarIdx>,
    f: Box<dyn FnOnce(&mut Storage) -> ()>,
}

impl<Storage: TStorage> Operation<Storage> {
    pub fn new(
        inputs: &[VarIdx],
        outputs: &[VarIdx],
        f: Box<dyn FnOnce(&mut Storage) -> ()>
    ) -> Self {
        Self {
            inputs: inputs.into(),
            outputs: outputs.into(),
            f,
        }
    }
}

impl<Storage: TStorage> TOperation<Storage> for Operation<Storage> {
    fn inputs(&self) -> &[VarIdx] {
        &self.inputs
    }

    fn outputs(&self) -> &[VarIdx] {
        &self.outputs
    }

    fn execute(self, storage: &mut Storage) {
        (self.f)(storage);
    }
}

pub trait TCircuitBuilder {
    type Constructed: TCircuitConstructed;
    type StorageBuilder: TStorageBuilder;
    type Operation: TOperation<<Self::StorageBuilder as TStorageBuilder>::Instance>;

    fn var<T>(&mut self) -> Var<T>;

    fn input<T>(&mut self, var: &Var<T>);

    fn output<T>(&mut self, var: &Var<T>);

    fn push(&mut self, op: Self::Operation);

    fn construct(self) -> Self::Constructed;
}

pub trait TCircuitConstructed {
    type Instance: TCircuitInstance;
    type ConstraintSystem;

    fn cs(&self) -> &Self::ConstraintSystem;
    
    fn spawn(&self) -> Self::Instance;
}

pub trait TCircuitInstance {

}

















struct TestCircuitBuilder {}

struct TestCircuitConstructed {}

struct TestCircuitInstance {}

struct TestConstraintSystem {}

struct TestStorageBuilder {}
struct TestStorage {}

impl TStorage for TestStorage {
    fn get<T: 'static>(&self, addr: Var<T>) -> Option<std::rc::Rc<T>> {
        todo!()
    }

    fn set<T: 'static >(&mut self, value: T, addr: Var<T>) -> Result<(), String> {
        todo!()
    }

    fn replace<T: 'static >(&mut self, value: T, addr: Var<T>) -> () {
        todo!()
    }
}

impl TStorageBuilder for TestStorageBuilder {
    type Instance = TestStorage;

    fn alloc<T: 'static>(&mut self) -> Var<T> {
        todo!()
    }

    fn instanciate(self) -> Self::Instance {
        todo!()
    }
}

impl TCircuitInstance for TestCircuitInstance {}

impl TCircuitConstructed for TestCircuitConstructed {
    type Instance = TestCircuitInstance;

    type ConstraintSystem = TestConstraintSystem;

    fn cs(&self) -> &Self::ConstraintSystem {
        todo!()
    }

    fn spawn(&self) -> Self::Instance {
        todo!()
    }
}

impl TCircuitBuilder for TestCircuitBuilder {
    type Constructed = TestCircuitConstructed;
    type StorageBuilder = TestStorageBuilder;
    type Operation = Operation<TestStorage>;

    fn var<T>(&mut self) -> Var<T> {
        todo!()
    }

    fn input<T>(&mut self, var: &Var<T>) {
        todo!()
    }

    fn output<T>(&mut self, var: &Var<T>) {
        todo!()
    }

    fn push(&mut self, op: Self::Operation) {
        todo!()
    }

    fn construct(self) -> Self::Constructed {
        todo!()
    }
}


fn test() {
    let mut storageBuilder = TestStorageBuilder{};
    let mut circuit = TestCircuitBuilder{};
    let x = circuit.var::<u32>();
    let y = circuit.var::<u32>();
    let z = circuit.var::<u32>();

    circuit.input(&x);
    circuit.input(&y);
    circuit.input(&z);
    
    let f = |x:u32, y:u32, z:u32| {(x + y, x - y, x + 2 * y, x)};

    let ans = circuit.var();
    circuit.output(&ans);
    
    let (_x, _y, _z, _ans) = (x.clone(), y.clone(), z.clone(), ans.clone());
    let closure = Box::new(move |storage: &mut TestStorage| {
        let _x = storage.get(_x).unwrap();
        let _y = storage.get(_y).unwrap();
        let _z = storage.get(_z).unwrap();
        storage.set(f(*_x, *_y, *_z), _ans).unwrap();
    });
    
    circuit.push(Operation::new(&[x.addr, y.addr, z.addr], &[ans.addr], closure));

    // circuit.output(&ans);

    let constructed = circuit.construct();
    let run = constructed.spawn();

}