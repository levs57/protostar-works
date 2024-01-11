// use std::{marker::PhantomData, rc::Rc};

// use ff::{PrimeField, Field};

// use crate::{circuit::{CircuitRun, PolyOp, ConstructedCircuit}, gate::Gate, witness::{ProtostarWtns, CSWtns}, constraint_system::ProtoGalaxyConstraintSystem};

// #[derive(Clone, Copy)]
// pub struct StorageAddr{
//     addr: usize,
// }

// pub trait Storable<T> {
//     fn get_addr(&self) -> StorageAddr;
// }

// pub type VarIdx = usize;

// #[derive(Copy, Clone)]
// pub struct Var<T> {
//     addr: VarIdx,
//     _marker: PhantomData<T>,
// }

// pub type SigIdx = usize;

// #[derive(Copy, Clone)]
// pub struct Sig<F> {
//     addr: SigIdx,
//     _marker: PhantomData<F>,
// }

// impl<T> Storable<T> for Var<T>{
//     fn get_addr(&self) -> StorageAddr {
//         StorageAddr {
//             typestate: SigTypeState::NeSig,
//             addr: self.addr
//         }
//     }
// }

// impl<T> Storable<T> for Sig<T>{
//     fn get_addr(&self) -> StorageAddr {
//         StorageAddr {
//             typestate: SigTypeState::Sig,
//             addr: self.addr
//         }
//     }
// }

// pub trait TStorage {
//     fn get<T: 'static, S: Storable<T>>(&self, storable: S) -> Option<Rc<T>>;
//     fn set<T: 'static, S: Storable<T>>(&mut self, value: T, storable: S) -> Result<(), String>;
//     fn replace<T: 'static, S: Storable<T>>(&mut self, value: T, storable: S) -> ();
// }

// pub trait TStorageBuilder {
//     type Instance: TStorage;

//     fn alloc<T: 'static>(&mut self) -> Var<T>;  // todo

//     fn instantiate(self) -> Self::Instance;
// }

// pub trait TOperation<Storage: TStorage> {
//     fn inputs(&self) -> &[StorageAddr];
//     fn outputs(&self) -> &[StorageAddr];
//     fn execute(self, storage: &mut Storage);
// }

// pub struct Operation<Storage: TStorage> {
//     inputs: Vec<StorageAddr>,
//     outputs: Vec<StorageAddr>,
//     f: Rc<dyn Fn(&mut Storage) -> ()>,
// }

// impl<Storage: TStorage> Operation<Storage> {
//     pub fn new(
//         inputs: &[StorageAddr],
//         outputs: &[StorageAddr],
//         f: Rc<dyn Fn(&mut Storage) -> ()>
//     ) -> Self {
//         Self {
//             inputs: inputs.to_vec(),
//             outputs: outputs.to_vec(),
//             f,
//         }
//     }
// }

// impl<Storage: TStorage> TOperation<Storage> for Operation<Storage> {
//     fn inputs(&self) -> &[StorageAddr] {
//         &self.inputs
//     }

//     fn outputs(&self) -> &[StorageAddr] {
//         &self.outputs
//     }

//     fn execute(self, storage: &mut Storage) {
//         (self.f)(storage);
//     }
// }

// pub trait TCommiter {
//     type Witness;
//     type Commitment;

//     fn commit(&mut self, witness: Self::Witness) -> Self::Commitment;
// }

// pub trait TCircuitBuilder {
//     type Field;
//     type Constructed: TCircuitConstructed;
//     type StorageBuilder: TStorageBuilder;
//     type Operation: TOperation<<Self::StorageBuilder as TStorageBuilder>::Instance>;

//     fn var<T>(&mut self) -> Var<T>;

//     fn sig(&mut self) -> Sig<Self::Field>;

//     fn input_sig(&mut self) -> Sig<Self::Field>;

//     fn output<T, S: Storable<T>>(&mut self, var: &S);

//     fn push(&mut self, op: Self::Operation);

//     fn construct(self) -> Self::Constructed;
// }

// pub trait TCircuitConstructed {
//     type Instance: TCircuitInstance;
//     type ConstraintSystem;

//     fn cs(&self) -> &Self::ConstraintSystem;
    
//     fn spawn(&self) -> Self::Instance;
// }

// pub trait TCircuitInstance {

// }

















// struct TestCircuitBuilder {}

// struct TestCircuitConstructed {}

// struct TestCircuitInstance {}

// struct TestConstraintSystem {}

// struct TestStorageBuilder {}
// struct TestStorage {}

// impl TStorage for TestStorage {
//     fn get<T: 'static, S: Storable<T>>(&self, storable: S) -> Option<Rc<T>> {
//         todo!()
//     }

//     fn set<T: 'static, S: Storable<T>>(&mut self, value: T, storable: S) -> Result<(), String> {
//         todo!()
//     }

//     fn replace<T: 'static, S: Storable<T>>(&mut self, value: T, storable: S) -> () {
//         todo!()
//     }
// }

// impl TStorageBuilder for TestStorageBuilder {
//     type Instance = TestStorage;

//     fn alloc<T: 'static>(&mut self) -> Var<T> {
//         todo!()
//     }

//     fn instantiate(self) -> Self::Instance {
//         todo!()
//     }
// }

// impl TCircuitInstance for TestCircuitInstance {}

// impl TCircuitConstructed for TestCircuitConstructed {
//     type Instance = TestCircuitInstance;

//     type ConstraintSystem = TestConstraintSystem;

//     fn cs(&self) -> &Self::ConstraintSystem {
//         todo!()
//     }

//     fn spawn(&self) -> Self::Instance {
//         todo!()
//     }
// }

// impl TCircuitBuilder for TestCircuitBuilder {
//     type Field = u128;
//     type Constructed = TestCircuitConstructed;
//     type StorageBuilder = TestStorageBuilder;
//     type Operation = Operation<TestStorage>;

//     fn var<T>(&mut self) -> Var<T> {
//         todo!()
//     }
//     fn sig(&mut self) -> Sig<Self::Field> {
//         todo!()
//     }


//     fn input_sig(&mut self) -> Sig<Self::Field> {
//         todo!()
//     }

//     fn output<T, S: Storable<T>>(&mut self, var: &S) {
//         todo!()
//     }

//     fn push(&mut self, op: Self::Operation) {
//         todo!()
//     }

//     fn construct(self) -> Self::Constructed {
//         todo!()
//     }
// }


// fn test() {
//     let mut circuit = TestCircuitBuilder{};
//     let x = circuit.input();
//     let y = circuit.input();
//     let z = circuit.input();

//     let f = |x:u32, y:u32, z:u32| {(x + y, x - y, x + 2 * y, x ^ z)};

//     let ans = circuit.var();
//     let closure = Rc::new(move |storage: &mut TestStorage| {
//         let _x = storage.get(x).unwrap();
//         let _y = storage.get(y).unwrap();
//         let _z = storage.get(z).unwrap();
//         storage.set(f(*_x, *_y, *_z), ans).unwrap();
//     });
//     circuit.push(Operation::new(&[x.get_addr(), y.get_addr(), z.get_addr()], &[ans.get_addr()], closure));
    
//     circuit.output(&ans);
    

//     let constructed = circuit.construct();
//     let run = constructed.spawn();

// }