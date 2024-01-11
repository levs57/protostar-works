pub trait TCircuitBuilderStorage {
    type Addr<T: ?Sized>;
    type ConstructedStorage: TCircuitSpawnerStorage;

    fn construct(self) -> Self::ConstructedStorage;
}

pub trait TCircuitSpawnerStorage {
    type RunStorage;

    fn spawn(&self) -> Self::RunStorage;
}

pub trait TCircuitRunStorage {
    type Addr<T: ?Sized>;
    type Witness;

    fn take_witness(&self) -> Self::Witness;
}

pub trait TWitness {
    type Addr<T: ?Sized>;
}

pub trait AllocatorOf<T>: TCircuitBuilderStorage {
    fn allocate(&mut self) -> Self::Addr<T>;
}


pub trait WriterOf<T>: TCircuitRunStorage
{
    fn put(&mut self, addr: Self::Addr<T>, val: T);
}

pub trait ReaderOf<T>: TCircuitRunStorage 
where 
    <Self as TCircuitRunStorage>::Witness: WitnessOf<T>
{
    fn get(&self, addr: Self::Addr<T>) -> T;
}

pub trait WitnessOf<T>: TWitness {
    fn get(&self, addr: Self::Addr<T>) -> T;
}