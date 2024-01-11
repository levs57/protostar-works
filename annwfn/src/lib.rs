#![feature(non_lifetime_binders)]


pub mod circuit;
pub mod committer;
pub mod storage;

#[cfg(test)]
pub mod tests {
    use super::*;
    use circuit::*;
    use storage::*;
    use committer::*;
    use std::marker::PhantomData;

    #[derive(Clone, Copy)]
    pub struct Var<T: ?Sized> {
        pub idx: usize,
        _phantom_data: PhantomData<T>,
    }

    impl<T: ?Sized> From<Addr<T>> for Var<T> {
        fn from(addr: Addr<T>) -> Self {
            Self {
                idx: addr.idx,
                _phantom_data: addr._phantom_data,
            }
        }
    }

    #[derive(Clone, Copy)]
    pub struct Sig<T: ?Sized> {
        pub idx: usize,
        _phantom_data: PhantomData<T>,
    }

    impl<T: ?Sized> From<Var<T>> for Sig<T> {
        fn from(var: Var<T>) -> Self {
            Self {
                idx: var.idx,
                _phantom_data: var._phantom_data,
            }
        }
    }

    pub struct Addr<T: ?Sized> {
        pub idx: usize,
        _phantom_data: PhantomData<T>,
    }

    impl<T> Clone for Addr<T> {
        fn clone(&self) -> Self {
            Self {
                idx: self.idx,
                _phantom_data: self._phantom_data,
            }
        }
    }

    impl<T> Copy for Addr<T> {}

    impl<T: ?Sized> From<Var<T>> for Addr<T> {
        fn from(v: Var<T>) -> Self {
            Self {
                idx: v.idx,
                _phantom_data: v._phantom_data,
            }
        }
    }

    impl<T: ?Sized> From<Sig<T>> for Addr<T> {
        fn from(v: Sig<T>) -> Self {
            Self {
                idx: v.idx,
                _phantom_data: v._phantom_data,
            }
        }
    }

    pub struct CircuitRunStorage {

    }

    impl CircuitRunStorage {

    }

    impl storage::TCircuitRunStorage for CircuitRunStorage {
        type Witness = Witness;
        type Addr<T: ?Sized> = Addr<T>;

        fn take_witness(&self) -> Self::Witness {
            todo!();
        }
    }

    impl ReaderOf<u64> for CircuitRunStorage {
        fn get(&self, addr: Self::Addr<u64>) -> u64 {
            todo!();
        }
    }

    impl WriterOf<u64> for CircuitRunStorage {
        fn put(&mut self, addr: Self::Addr<u64>, val: u64) {
            todo!();
        }
    }

    impl ReaderOf<bool> for CircuitRunStorage {
        fn get(&self, addr: Self::Addr<bool>) -> bool {
            todo!();
        }
    }

    impl WriterOf<bool> for CircuitRunStorage {
        fn put(&mut self, addr: Self::Addr<bool>, val: bool) {
            todo!();
        }
    }

    pub struct Witness {

    }
    
    impl TWitness for Witness {
        type Addr<T: ?Sized> = Addr<T>;
    }

    impl WitnessOf<u64> for Witness {
        fn get(&self, addr: Self::Addr<u64>) -> u64 {
            todo!();
        }
    }

    impl WitnessOf<bool> for Witness {
        fn get(&self, addr: Self::Addr<bool>) -> bool {
            todo!();
        }
    }

    pub struct CircuitRun {

    }

    impl CircuitRun {
        pub fn new() -> Self {
            Self {}
        }
    }

    impl circuit::TCircuitRun for CircuitRun {
        type Addr<T: ?Sized> = Addr<T>;
        type Storage = CircuitRunStorage;
        type Witness = Witness;
    
        fn storage(&mut self) -> &mut Self::Storage {
            todo!();
        }
        
        fn execute(&mut self) {
            todo!();
        }
        
        fn set_input<T>(&mut self, addr: Self::Addr<T>, val: T) {
            todo!();
        }
        
    }

    pub struct CircuitSpawner {

    }

    impl CircuitSpawner {
        pub fn new() -> Self {
            Self {}
        }
    }

    pub struct CircuitSpawnerStorage {

    }

    impl storage::TCircuitSpawnerStorage for CircuitSpawnerStorage {
        type RunStorage = CircuitRunStorage;

        fn spawn(&self) -> Self::RunStorage {
            todo!();
        }
    }

    impl circuit::TCircuitRunSpawner for CircuitSpawner {
        type RunStorage = CircuitRunStorage;
        type CircuitRun = CircuitRun;
        type Storage = CircuitSpawnerStorage;
    
        fn spawn(&self) -> Self::CircuitRun {
            todo!();
        }
    }

    pub struct CircuitBuilder {

    }

    impl CircuitBuilder {
        pub fn new() -> Self {
            Self {}
        }

        pub fn allocate_var<T>(&mut self) -> <Self as circuit::TCircuitBuilder>::Var<T>
        where <Self as TCircuitBuilder>::Storage: AllocatorOf<T> {
            Var::from(self.storage().allocate())
        }

        pub fn allocate_sig<ST, CGT>(&mut self, commitment_addr: <<Self as TCircuitBuilder>::Commiter as TCommitmentScheme>::CommitmentGrpAddr<CGT>) -> <Self as circuit::TCircuitBuilder>::Sig<ST>
        where 
            <Self as TCircuitBuilder>::Storage: AllocatorOf<ST>,
            <Self as TCircuitBuilder>::Commiter: TCommitmentSchemeWith<CGT>,
            CGT: committer::CommitTo<ST>,
            CGT: for <T> committer::TCommitmentGroup<Var<T> = <Self as TCircuitBuilder>::Var<T>, Sig<T> = <Self as TCircuitBuilder>::Sig<T>>
        {
            let var = self.allocate_var();
            self.committer().get_group(commitment_addr).commit(var)
        }
    }

    pub struct Execution {}

    impl circuit::TExecution for Execution {
        fn advice(&mut self) {}
    }

    pub struct ConstraintSystem {}

    impl circuit::TConstraintSystem for ConstraintSystem {
        type Field = u64;
        type Sig<T: ?Sized> = Sig<T>;
        type Witness = Witness;

        fn constrain(&mut self, constraint: Box<dyn TConstraint<Field = Self::Field, Witness=Self::Witness>>) {
            todo!();
        }
    }

    pub struct StorageBuilder {
        u64_idx: usize,
        bool_idx: usize,
    }

    impl storage::TCircuitBuilderStorage for StorageBuilder {
        type Addr<T: ?Sized> = Addr<T>;
        type ConstructedStorage = CircuitSpawnerStorage;
    
        fn construct(self) -> Self::ConstructedStorage {
            todo!();
        }
    }

    impl AllocatorOf<u64> for StorageBuilder {
        fn allocate(&mut self) -> Self::Addr<u64> {
            self.u64_idx += 1;
            Addr {
                idx: (self.u64_idx - 1),
                _phantom_data: PhantomData,
            }
        }
    }

    impl AllocatorOf<bool> for StorageBuilder {
        fn allocate(&mut self) -> Self::Addr<bool> {
            self.bool_idx += 1;
            Addr {
                idx: (self.bool_idx - 1),
                _phantom_data: PhantomData,
            }
        }
    }

    pub struct Commiter {

    }

    impl committer::TCommitmentScheme for Commiter {
        type CommitmentGrpAddr<T> = Addr<T>;
    }

    impl circuit::TCircuitBuilder for CircuitBuilder {
        type Field = u64;
        type Sig<T: ?Sized> = Sig<T>;
        type Var<T: ?Sized> = Var<T>;
        type ConstructedCircuit = CircuitSpawner;
        type Storage = StorageBuilder;
        type ConstraintSystem = ConstraintSystem;
        type Execution = Execution;
        type Commiter = Commiter;
    
        fn storage(&mut self) -> &mut Self::Storage {
            todo!();
        }

        fn cs(&mut self) -> &mut Self::ConstraintSystem {
            todo!();
        }
        
        fn execution(&mut self) -> &mut Self::Execution {
            todo!();
        }

        fn committer(&mut self) -> &mut Self::Commiter {
            todo!();
        }
        
        fn construct(self) -> Self::ConstructedCircuit {
            todo!();
        }
    }
    

    fn branching_exec<T>(t: T, f: T, cond: bool) -> T {
        return if cond {
            t
        } else {
            f
        }
    }

    fn branching_constraint<T: Eq>(t: T, f: T, cond: bool, res: T) -> bool {
        return res == if cond {
            t
        } else {
            f
        }
    }

    struct SomeCommittmentGroup {

    }

    impl SomeCommittmentGroup {
        pub fn new() -> Self {
            Self {}
        }
    }

    impl committer::CommitTo<u64> for SomeCommittmentGroup {
        fn commit(&mut self, var: Self::Var<u64>) -> Self::Sig<u64> {
            todo!()
        }
    }

    impl committer::CommitTo<bool> for SomeCommittmentGroup {
        fn commit(&mut self, var: Self::Var<bool>) -> Self::Sig<bool> {
            todo!()
        }
    }

    impl committer::TCommitmentGroup for SomeCommittmentGroup {
        type Var<T: ?Sized> = Var<T>;
        type Sig<T: ?Sized> = Sig<T>;
        type Commitment = u64;

        fn calculate(&mut self) -> Self::Commitment {
            todo!()
        }
    }

    impl committer::TCommitmentSchemeWith<SomeCommittmentGroup> for Commiter {
        fn add_group(&mut self, grp: SomeCommittmentGroup) -> Self::CommitmentGrpAddr<SomeCommittmentGroup> {
            todo!()
        }

        fn get_group(&mut self, addr: Self::CommitmentGrpAddr<SomeCommittmentGroup>) -> &mut SomeCommittmentGroup {
            todo!()
        }
        
    }

    struct Constraint {
        f: Box<dyn Fn(&<Self as circuit::TConstraint>::Witness) -> Vec<<Self as circuit::TConstraint>::Field>>,
    }

    impl Constraint {
        fn new(f: Box<dyn Fn(&<Self as circuit::TConstraint>::Witness) -> Vec<<Self as circuit::TConstraint>::Field>>) -> Self {
            Self {
                f,
            }
        }
    }

    impl circuit::TConstraint for Constraint {
        type Field = u64;
        type Witness = Witness;

        fn execute(&self, wtns: &Self::Witness) -> Vec<Self::Field> {
            (self.f)(wtns)
        }
    }

    #[test]
    pub fn test_pipeline() {
        let mut builder = CircuitBuilder::new();
        let l0_grp_addr = builder.committer().add_group(SomeCommittmentGroup::new());

        let u_x = builder.allocate_sig(l0_grp_addr);
        let u_y = builder.allocate_sig(l0_grp_addr);
        let b_c = builder.allocate_sig(l0_grp_addr);

        let u_r: Sig<u64> = builder.allocate_sig(l0_grp_addr);
        builder.execution().advice();

        builder.cs().constrain(Box::new(Constraint::new(Box::new(move |storage: &Witness| {
            let _u_x = storage.get(u_x.into());
            let _u_y = storage.get(u_y.into());
            let _b_c = storage.get(b_c.into());
            let _u_r = storage.get(u_r.into());
            
            let f = |x: u64, y: u64, c: bool| -> u64 {
                if c {
                    x
                } else {
                    y
                }
            };
            let _res = f(_u_x, _u_y, _b_c);

            vec![_res - _u_r]
        }))));
    }
}