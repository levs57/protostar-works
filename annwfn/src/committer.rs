pub trait TCommitmentScheme {
    type CommitmentGrpAddr<T>: for <T> Clone + Copy;
}

pub trait TCommitmentSchemeWith<CommitmentGroup>: TCommitmentScheme {
    fn add_group(&mut self, grp: CommitmentGroup) -> Self::CommitmentGrpAddr<CommitmentGroup>;
    fn get_group(&mut self, addr: Self::CommitmentGrpAddr<CommitmentGroup>) -> &mut CommitmentGroup;
}

pub trait TCommitmentGroup {
    type Var<T: ?Sized>;
    type Sig<T: ?Sized>;
    type Commitment;

    fn calculate(&mut self) -> Self::Commitment;
}

pub trait CommitTo<T>: TCommitmentGroup {
    fn commit(&mut self, var: Self::Var<T>) -> Self::Sig<T>;
}