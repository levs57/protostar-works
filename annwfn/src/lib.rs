use std::{marker::PhantomData, cell::OnceCell, ops::Deref, fmt::Debug, vec, iter::Once};

use storage::{ReaderOf, AllocatorOf, WriterOf, TStorageBuilder};

pub mod storage;

pub struct Sig<T> {
    idx: usize,
    _pd: PhantomData<T>, 
}

impl<T> Clone for Sig<T> {
    fn clone(&self) -> Self {
        Self { idx: self.idx.clone(), _pd: self._pd.clone() }
    }
}

impl<T> Copy for Sig<T> {}

impl<T> Sig<T> {
    pub fn new(idx: usize) -> Self {
        Self {
            idx,
            _pd: PhantomData,
        }
    }
}

pub struct StorageBuilder {
    b_idx: usize,
    u64_idx: usize,
}

impl StorageBuilder {
    fn new() -> Self {
        Self {
            b_idx: 0,
            u64_idx: 0,
        }
    }    
}

impl storage::TStorageBuilder for StorageBuilder {
    type RunStorage = Witness;

    fn spawn(&self) -> Self::RunStorage {
        Self::RunStorage {
            bools: Vec::with_capacity(self.b_idx),
            u64s: Vec::with_capacity(self.u64_idx),
        }
    }
}

impl AllocatorOf<bool> for StorageBuilder {
    fn allocate(&mut self) -> <Self::RunStorage as storage::TRunStorage>::Addr<bool> {
        self.b_idx += 1;
        Sig::<bool> {
            idx: self.b_idx - 1,
            _pd: PhantomData,
        }
    }
}

impl AllocatorOf<u64> for StorageBuilder {
    fn allocate(&mut self) -> <Self::RunStorage as storage::TRunStorage>::Addr<u64> {
        self.u64_idx += 1;
        Sig::<u64> {
            idx: self.u64_idx - 1,
            _pd: PhantomData,
        }
    }
}

pub struct Witness {
    bools: Vec<Option<bool>>,
    u64s: Vec<Option<u64>>,
}

impl storage::TRunStorage for Witness {
    type Addr<T> = Sig<T>;
}

impl storage::WriterOf<bool> for Witness {
    fn put(&mut self, addr: &Self::Addr<bool>, val: bool) {
        self.bools[addr.idx] = Some(val);
    }
}

impl storage::ReaderOf<bool> for Witness {
    fn get(&self, addr: &Self::Addr<bool>) -> &bool {
        self.bools.get(addr.idx).unwrap().as_ref().expect("Reading uninitialized signal")
    }
} 

impl storage::WriterOf<u64> for Witness {
    fn put(&mut self, addr: &Self::Addr<u64>, val: u64) {
        self.u64s[addr.idx] = Some(val);
    }
}

impl storage::ReaderOf<u64> for Witness {
    fn get(&self, addr: &Self::Addr<u64>) -> &u64 {
        self.u64s.get(addr.idx).unwrap().as_ref().expect("Reading uninitialized signal")
    }
} 

enum BuildVar<T> {
    V,
    S(Sig<T>)
}

impl<T> Clone for BuildVar<T> {
    fn clone(&self) -> Self {
        match self {
            Self::V => Self::V,
            Self::S(arg0) => Self::S(arg0.clone()),
        }
    }
} 

impl<T> Copy for BuildVar<T> {}

impl<T> From<Sig<T>> for BuildVar<T> {
    fn from(value: Sig<T>) -> Self {
        Self::S(value)
    }
}

enum RunVar<T> {
    V(OnceCell<T>),
    S(Sig<T>)
}

impl<T> From<BuildVar<T>> for RunVar<T> {
    fn from(var: BuildVar<T>) -> Self {
        match var {
            BuildVar::V => Self::V(OnceCell::new()),
            BuildVar::S(s) => Self::S(s),
        }
    } 
}

impl<T: Debug> RunVar<T>
where Witness: ReaderOf<T> {
    fn get_from<'storage, 'addr: 'storage>(&'addr self, storage: &'storage Witness) -> &'storage T {
        match self {
            RunVar::V(cell) => return cell.get().expect("Reading uninitialized variable"),
            RunVar::S(sig) => return storage.get(sig),
        }
    }
}

impl<T: Debug> RunVar<T>
where Witness: WriterOf<T> {
    fn set_to(&self, storage: &mut Witness, value: T) {
        match self {
            RunVar::V(cell) => cell.set(value).expect("Second write to variable"),
            RunVar::S(sig) => storage.put(sig, value),
        }
    }
}


struct GenericCommitmentGroup {
    u64_addrs: Vec<Sig<u64>>,
    bool_addrs: Vec<Sig<bool>>,
}

impl GenericCommitmentGroup {
    fn new() -> Self {
        Self {
            u64_addrs: vec![],
            bool_addrs: vec![],
        }
    }
}

pub trait Commitment {
    type SeedType;
    type CommitmentType;

    fn get(&self, storage: &Witness, seed: Self::SeedType) -> Self::CommitmentType;
}

impl Commitment for GenericCommitmentGroup {
    type SeedType = u64;
    type CommitmentType = u64;

    fn get(&self, storage: &Witness, seed: Self::SeedType) -> Self::CommitmentType {
        let mut running = 0;
        let mut pow = 1;
        for addr in &self.u64_addrs {
            running += pow * storage.get(&addr);
            pow *= seed;
        }
        for addr in &self.bool_addrs {
            running += pow * u64::from(*storage.get(&addr));
            pow *= seed;
        }
        running
    }
}

pub trait CommitmentGroupOf<T>
where StorageBuilder: AllocatorOf<T> {
    fn create(&mut self, storage: &mut StorageBuilder) -> Sig<T>;
}

impl CommitmentGroupOf<bool> for GenericCommitmentGroup {
    fn create(&mut self, storage: &mut StorageBuilder) -> Sig<bool> {
        self.bool_addrs.push(storage.allocate());
        *self.bool_addrs.last().unwrap()
    }
}

impl CommitmentGroupOf<u64> for GenericCommitmentGroup {
    fn create(&mut self, storage: &mut StorageBuilder) -> Sig<u64> {
        self.u64_addrs.push(storage.allocate());
        *self.u64_addrs.last().unwrap()
    }
}

struct OtherCommitmentType {}

impl Commitment for OtherCommitmentType {
    type SeedType = u64;

    type CommitmentType = u64;

    fn get(&self, storage: &Witness, seed: Self::SeedType) -> Self::CommitmentType {
        seed
    }
}

#[derive(Clone, Copy)]
pub struct CGAddr<T> {
    idx: usize,
    _pd: PhantomData<T>, 
}

impl<T> CGAddr<T> {
    pub fn new(idx: usize) -> Self {
        Self {
            idx,
            _pd: PhantomData,
        }
    }
}

struct CommitmentScheme {
    generic: Vec<GenericCommitmentGroup>,
    other: OtherCommitmentType,
}

impl CommitmentScheme {
    fn new() -> Self {
        Self {
            generic: vec![],
            other: OtherCommitmentType {},
        }
    }
}

pub trait TCommitmentScheme {
    type SeedType;
    type CommitmentType;
    type Addr<T>;
}

impl TCommitmentScheme for CommitmentScheme {
    type SeedType = u64;
    type CommitmentType = u64;
    type Addr<T> = CGAddr<T>;
}

pub trait CommitmentSchemeWith<GRP>: TCommitmentScheme
where GRP: Commitment<SeedType = Self::SeedType, CommitmentType = Self::CommitmentType>,  {
    fn create(&mut self) -> Self::Addr<GRP>;
    fn get(&mut self, addr: &Self::Addr<GRP>) -> &mut GRP;
}

impl CommitmentSchemeWith<GenericCommitmentGroup> for CommitmentScheme {
    fn create(&mut self) -> Self::Addr<GenericCommitmentGroup> {
        self.generic.push(GenericCommitmentGroup::new());
        <Self as TCommitmentScheme>::Addr::new(self.generic.len() - 1)
    }

    fn get(&mut self, addr: &Self::Addr<GenericCommitmentGroup>) -> &mut GenericCommitmentGroup {
        self.generic.get_mut(addr.idx).expect("Getting non existent Commitment group")
    }

}

impl CommitmentSchemeWith<OtherCommitmentType> for CommitmentScheme {
    fn create(&mut self) -> Self::Addr<OtherCommitmentType> {
        <Self as TCommitmentScheme>::Addr::new(0)
    }

    fn get(&mut self, addr: &Self::Addr<OtherCommitmentType>) -> &mut OtherCommitmentType {
        &mut self.other
    }

}


pub trait TConstraint {
    type Field;
    type Witness;

    fn evaluate(&self, witness: &Self::Witness) -> Vec<Self::Field>;
}


struct ConstraintSystem {
    constraints: Vec<Box<dyn TConstraint<Field = <Self as TConstraintSystem>::Field, Witness = <Self as TConstraintSystem>::Witness>>>,
}

impl ConstraintSystem {
    pub fn new() -> Self {
        Self {
            constraints: vec![],
        }
    }
}

pub trait TConstraintSystem {
    type Field;
    type Witness;

    fn add(&mut self, constraint: Box<dyn TConstraint<Field = <Self as TConstraintSystem>::Field, Witness = <Self as TConstraintSystem>::Witness>>);

    fn crossterms(witness: &Witness) -> Self::Witness;
}

impl TConstraintSystem for ConstraintSystem {
    type Field = u64;
    type Witness = Witness;

    fn crossterms(witness: &Witness) -> Self::Witness {
        todo!()
    }

    fn add(&mut self, constraint: Box<dyn TConstraint<Field = <Self as TConstraintSystem>::Field, Witness = <Self as TConstraintSystem>::Witness>>) {
        self.constraints.push(constraint)
    }
}

struct Execution {
    steps: Vec<Box<dyn TComputation>>
}

struct ExecutionBuilder {
    steps: Vec<Box<dyn TComputationTemplate>>
}

impl Execution {
    pub fn new(steps: Vec<Box<dyn TComputation>>) -> Self {
        Self {
            steps,
        }
    }
}

pub trait TComputationTemplate {
    fn spawn(&self) -> Box<dyn TComputation>;
}

pub trait TComputation {
    fn execute(&self, storage: &mut Witness);
}

pub trait TExecutionBuilder {
    fn add(&mut self, computation: Box<dyn TComputationTemplate>);

    fn spawn(&self) -> Execution; 
}

impl TExecutionBuilder for ExecutionBuilder {
    fn add(&mut self, computation: Box<dyn TComputationTemplate>) {
        self.steps.push(computation)
    }

    fn spawn(&self) -> Execution {
        Execution::new(self.steps.iter().map(|step| step.spawn()).collect())
    }
}

impl ExecutionBuilder {
    pub fn new() -> Self {
        Self {
            steps: vec![],
        }
    }
}


struct CircuitBuilder {
    storage: StorageBuilder,
    commitment_scheme: CommitmentScheme,
    constraint_system: ConstraintSystem,
    execution: ExecutionBuilder,
}

impl CircuitBuilder {
    pub fn new() -> Self {
        Self {
            storage: StorageBuilder::new(),
            commitment_scheme: CommitmentScheme::new(),
            constraint_system: ConstraintSystem::new(),
            execution: ExecutionBuilder::new(),
        }
    }

    pub fn new_commitment_group<GRP>(&mut self) -> <CommitmentScheme as TCommitmentScheme>::Addr<GRP>
    where 
        GRP: Commitment<SeedType = <CommitmentScheme as TCommitmentScheme>::SeedType, CommitmentType = <CommitmentScheme as TCommitmentScheme>::CommitmentType>,
        CommitmentScheme: CommitmentSchemeWith<GRP> {
        CommitmentSchemeWith::<GRP>::create(&mut self.commitment_scheme)
    }

    pub fn new_signal<T, GRP>(&mut self, grp_addr: &<CommitmentScheme as TCommitmentScheme>::Addr<GRP>) -> Sig<T>
    where 
        GRP: Commitment<SeedType = <CommitmentScheme as TCommitmentScheme>::SeedType, CommitmentType = <CommitmentScheme as TCommitmentScheme>::CommitmentType>,
        GRP: CommitmentGroupOf<T>,
        CommitmentScheme: CommitmentSchemeWith<GRP>,
        StorageBuilder: AllocatorOf<T>, {
        self.commitment_scheme.get(grp_addr).create(&mut self.storage)
    }

    pub fn set_constraint(&mut self, constraint: Box<dyn TConstraint<Field = <ConstraintSystem as TConstraintSystem>::Field, Witness = <ConstraintSystem as TConstraintSystem>::Witness>>) {
        self.constraint_system.add(constraint)
    }

    pub fn add_step(&mut self, step: Box<dyn TComputationTemplate>) {
        self.execution.add(step)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;


    fn foo(a: &u64, b: &u64, c: &bool, d: &u64) -> Vec<u64> {
        vec![(if *c {*a} else {*b}) - *d]
    }

    fn bar(a: &u64, b: &u64, c: &bool) -> (u64, bool) {
        ((if *c {*a} else {*b}), *a < *b)
    }

    #[test]
    fn it_works() {
        // assign!(builder, [d, e] <-- bar(a, b, c))  // will create 2 variables d and e and add execution step to assign them with return values of bar(a, b, c)
        // assign!(builder, d <-- bar(a, b, c)) // same as assign!(builder, [d] <-- bar(a, b, c))
        // assign!(builder, [d, e] <-- bar(a, b, c) | foo) // same as above but add a constraint foo(a, b, c, d, e)
        // assign!(builder, [d, e] <== bar(a, b, c)) // same as above, but constraint will be constructed ad-hoc as [bar(a, b, c).0 - d, bar(a, b, c).1 - e] 

        let mut builder = CircuitBuilder::new();
        let round_0_commitment = builder.new_commitment_group::<GenericCommitmentGroup>();
        let s1 = builder.new_signal::<u64, _>(&round_0_commitment);
        let s2 = builder.new_signal::<u64, _>(&round_0_commitment);
        let b1 = builder.new_signal::<bool, _>(&round_0_commitment);
        let s3 = builder.new_signal::<u64, _>(&round_0_commitment);
        let s4 = builder.new_signal::<bool, _>(&round_0_commitment);

        struct _Computation<T1, T2, T3, T4, T5> {
            s1: RunVar<T1>,
            s2: RunVar<T2>,
            b1: RunVar<T3>,
            f: Rc<dyn Fn(&T1, &T2, &T3) -> (T4, T5)>,
            s3: RunVar<T4>,
            s4: RunVar<T5>,
        }

        struct _ComputationTemplate<T1, T2, T3, T4, T5> {
            s1: BuildVar<T1>,
            s2: BuildVar<T2>,
            b1: BuildVar<T3>,
            f: Rc<dyn Fn(&T1, &T2, &T3) -> (T4, T5)>,
            s3: BuildVar<T4>,
            s4: BuildVar<T5>,
        }

        impl<T1, T2, T3, T4, T5> TComputationTemplate for _ComputationTemplate<T1, T2, T3, T4, T5> 
        where 
            Witness: storage::ReaderOf<T1> + storage::ReaderOf<T2> + storage::ReaderOf<T3> + storage::WriterOf<T4> + storage::WriterOf<T5>,
            T1: Debug + 'static,
            T2: Debug + 'static,
            T3: Debug + 'static, 
            T4: Debug + 'static, 
            T5: Debug + 'static,
        {
            fn spawn(&self) -> Box<dyn TComputation> {
                Box::new(_Computation {
                    s1: self.s1.into(),
                    s2: self.s2.into(),
                    b1: self.b1.into(),
                    f: self.f.clone(),
                    s3: self.s3.into(),
                    s4: self.s4.into(),
                })
            }
        }

        impl<T1, T2, T3, T4, T5> TComputation for _Computation<T1, T2, T3, T4, T5>
        where 
            Witness: storage::ReaderOf<T1> + storage::ReaderOf<T2> + storage::ReaderOf<T3> + storage::WriterOf<T4> + storage::WriterOf<T5>,
            T1: Debug,
            T2: Debug,
            T3: Debug, 
            T4: Debug, 
            T5: Debug,
        {
            fn execute(&self, storage: &mut Witness) {
                let (_s3, _s4) = (self.f)(self.s1.get_from(storage), self.s2.get_from(storage), self.b1.get_from(storage));
                self.s3.set_to(storage, _s3);
                self.s4.set_to(storage, _s4);
            }
        }

        builder.add_step(Box::new(_ComputationTemplate {
            s1: s1.into(),
            s2: s2.into(),
            b1: b1.into(),
            f: Rc::new(bar),
            s3: s3.into(),
            s4: s4.into(),
        }));

        struct _Constraint<T1, T2, T3, T4, T5> {
            s1: Sig<T1>,
            s2: Sig<T2>,
            b1: Sig<T3>,
            s3: Sig<T4>,
            s4: Sig<T5>,
            f: Box<dyn Fn(&T1, &T2, &T3, &T4, &T5) -> Vec<u64>>,
        }

        impl<T1, T2, T3, T4, T5> TConstraint for _Constraint<T1, T2, T3, T4, T5> 
        where 
            Witness: storage::ReaderOf<T1> + storage::ReaderOf<T2> + storage::ReaderOf<T3> + storage::ReaderOf<T4> + storage::ReaderOf<T5>,
        {
            type Field = u64;

            type Witness = Witness;

            fn evaluate(&self, witness: &Self::Witness) -> Vec<Self::Field> {
                (self.f)(witness.get(&self.s1), witness.get(&self.s2), witness.get(&self.b1), witness.get(&self.s3), witness.get(&self.s4))
            }
        }
        
        builder.set_constraint(Box::new(_Constraint {
            s1, s2, b1, s3, s4,
            f: Box::new(|&s1, &s2, &b1, &s3, &s4| {
                let (_s3, _s4) = bar(&s1, &s2, &b1);
                vec![(u64::from(_s3) - u64::from(s3)), (u64::from(_s4) - u64::from(s4))]
            })
        }));


    }
}
