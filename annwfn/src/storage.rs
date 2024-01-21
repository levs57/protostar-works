pub trait TStorageBuilder {
    type RunStorage: TRunStorage;

    fn spawn(&self) -> Self::RunStorage;
}

pub trait TRunStorage {
    type Addr<T>;
}

pub trait AllocatorOf<T>: TStorageBuilder {
    fn allocate(&mut self) -> <Self::RunStorage as TRunStorage>::Addr<T>;
}

pub trait WriterOf<T>: TRunStorage {
    fn put(&mut self, addr: &Self::Addr<T>, val: T);
}

pub trait ReaderOf<T>: TRunStorage {
    fn get(&self, addr: &Self::Addr<T>) -> &T;
}
