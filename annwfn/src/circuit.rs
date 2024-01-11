use crate::storage::{TCircuitBuilderStorage, TCircuitSpawnerStorage, TCircuitRunStorage};
use crate::committer::{TCommitmentScheme};

//                    IO
//                    ↕
// Build -> Spawn -> Run -> Fold
//                    ↕
//                  Commit

pub trait TExecution {
    fn advice(&mut self);
}

pub trait TConstraint {
    type Field;
    type Witness;
    fn execute(&self, wtns: &Self::Witness) -> Vec<Self::Field>;
}

pub trait TConstraintSystem {
    type Field;
    type Witness;
    type Sig<T: ?Sized>;

    fn constrain(&mut self, constraint: Box<dyn TConstraint<Field = Self::Field, Witness=Self::Witness>>);
}

pub trait TCircuitBuilder {
    type Field;
    type Sig<T: ?Sized>;
    type Var<T: ?Sized>;
    type ConstructedCircuit;
    type Storage: TCircuitBuilderStorage;
    type Commiter: TCommitmentScheme;
    type ConstraintSystem: for <T> TConstraintSystem<Field=Self::Field, Sig<T> = Self::Sig<T>>;
    type Execution: TExecution;

    fn storage(&mut self) -> &mut Self::Storage;
    fn cs(&mut self) -> &mut Self::ConstraintSystem;
    fn execution(&mut self) -> &mut Self::Execution;
    fn committer(&mut self) -> &mut Self::Commiter;

    fn construct(self) -> Self::ConstructedCircuit;
}

pub trait TCircuitRunSpawner {
    type RunStorage;
    type CircuitRun: TCircuitRun<Storage=Self::RunStorage>;
    type Storage: TCircuitSpawnerStorage<RunStorage=Self::RunStorage>;

    fn spawn(&self) -> Self::CircuitRun;
}

pub trait TCircuitRun {
    type Addr<T: ?Sized>;
    type Storage: for <T> TCircuitRunStorage<Addr<T> = Self::Addr<T>>;
    type Witness;

    fn storage(&mut self) -> &mut Self::Storage;
    fn execute(&mut self);
    fn set_input<T>(&mut self, addr: Self::Addr<T>, val: T);
}
