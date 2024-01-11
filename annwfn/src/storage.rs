pub trait TCircuitBuilderStorage {
    type Addr<T: ?Sized>;
    type ConstructedStorage: TCircuitSpawnerStorage;

    fn construct(self) -> Self::ConstructedStorage;
}

pub trait AllocatorOf<T>: TCircuitBuilderStorage {
    fn allocate(&mut self) -> Self::Addr<T>;
}

pub trait TCircuitSpawnerStorage {
    type RunStorage;

    fn spawn(&self) -> Self::RunStorage;
}

pub trait TCircuitRunStorage {
    type Addr<T: ?Sized>;
}

pub trait WriterOf<T>: TCircuitRunStorage {
    fn put(&mut self, addr: Self::Addr<T>, val: T);
}

pub trait ReaderOf<T>: TCircuitRunStorage {
    fn get(&self, addr: Self::Addr<T>) -> T;
}