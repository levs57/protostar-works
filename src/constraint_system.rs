use std::marker::PhantomData;

use ff::PrimeField;

use crate::gate::Gate;

/// Constraint commitment kind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommitKind {
    Trivial,
    Group,
    Zero, // Used in cases where we do not need to commit.
}

/// Variable descriptor. We treat challenges as public variables
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Visibility {
    Public,
    Private,
}

/// A variable inside a constraint system.
/// 
/// Variables are what constraints operate on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Variable {
    pub visibility: Visibility,
    pub round: usize,
    pub index: usize,
}

/// A polynomial constraint.
/// 
/// It is fully described by a polynomial (gate) and a list of variables it attaches to.
#[derive(Debug, Clone)]
pub struct Constraint<F: PrimeField, G: Gate<F>>{
    pub inputs: Vec<Variable>,
    pub gate: G,
    _marker: PhantomData<F>,
}

/// Constraints are grouped by their CommitKind.
/// 
/// Currently this struct has some additional information. This will probably
/// be moved in the near future
#[derive(Debug, Clone)]
struct ConstraintGroup<F: PrimeField, G: Gate<F>>{
    pub entries: Vec<Constraint<F, G>>,
    pub kind: CommitKind,
    pub num_rhs: usize,
    pub max_degree: usize,
}

impl<F: PrimeField, G: Gate<F>> ConstraintGroup<F, G> {
    pub fn new(kind: CommitKind, max_degree: usize) -> Self {
        Self {
            entries: Default::default(),
            kind,
            num_rhs: Default::default(),
            max_degree,
        }
    }

    pub fn constrain(&mut self, inputs: &[Variable], gate: G) {
        assert!(gate.d() <= self.max_degree, "Constraint degree is too large for this group.");
        assert!(gate.i() == inputs.len(), "Invalid amount of arguments supplied.");

        self.num_rhs += gate.o();
        self.entries.push(Constraint{inputs : inputs.to_vec(), gate, _marker : PhantomData});
    }
}

/// Round witness shape specification: the amount of public and private variables respectively
/// 
/// Any witness used for this constraint system has to at least comply with the spec.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct RoundWitnessSpec(pub usize, pub usize);

/// Witness shape specification: a collection of specifications for each round
/// 
/// Any witness used for this constraint system has to at least comply with the spec.
pub type WitnessSpec = Vec<RoundWitnessSpec>;

pub trait CS<F: PrimeField, G: Gate<F>> {
    fn num_rounds(&self) -> usize;

    fn last_round(&self) -> usize {
        self.num_rounds() - 1
    }

    fn new_round(&mut self);

    fn witness_spec(&self) -> &WitnessSpec;

    fn alloc_in_round(&mut self, round: usize, visibility: Visibility, size: usize) -> Vec<Variable>;

    fn alloc(&mut self, visibility: Visibility, size: usize) -> Vec<Variable> {
        self.alloc_in_round(self.last_round(), visibility, size)
    }

    fn constrain(&mut self, kind: CommitKind, inputs: &[Variable], gate: G);
}

#[derive(Debug, Clone)]
pub struct ConstraintSystem<F: PrimeField, G: Gate<F>> {
    spec: WitnessSpec,
    constraint_groups: [ConstraintGroup<F, G>; 3],
}

impl<F: PrimeField, G: Gate<F>> ConstraintSystem<F, G> {
    pub fn new(num_rounds: usize, max_degree: usize) -> Self {
        let constraint_groups = [
            ConstraintGroup::new(CommitKind::Trivial, 0),  // FIXME: correct max_degree
            ConstraintGroup::new(CommitKind::Group, max_degree),
            ConstraintGroup::new(CommitKind::Zero, 1),
        ];

        Self {
            spec: vec![RoundWitnessSpec::default(); num_rounds],
            constraint_groups,
        }
    }

    /// A (short-lived) cursor to the constraint group of a given kind
    fn constraint_group(&mut self, kind: CommitKind) -> &mut ConstraintGroup<F, G> {
        match kind {
            CommitKind::Trivial => &mut self.constraint_groups[0],
            CommitKind::Group => &mut self.constraint_groups[1],
            CommitKind::Zero => &mut self.constraint_groups[2],
        }
    }

    // would love to add this to the trait, but crab god said not yet
    // https://github.com/rust-lang/rust/issues/91611
    pub fn iter_constraints(&self) -> impl Iterator<Item = &Constraint<F, G>> {
        self.constraint_groups.iter().flat_map(|cg| cg.entries.iter())
    }
}

impl<F: PrimeField, G: Gate<F>> CS<F, G> for ConstraintSystem<F, G> {
    fn num_rounds(&self) -> usize {
        self.spec.len()
    }

    fn new_round(&mut self) {
        self.spec.push(RoundWitnessSpec::default())
    }

    fn witness_spec(&self) -> &WitnessSpec {
        &self.spec
    }

    fn alloc_in_round(&mut self, round: usize, visibility: Visibility, size: usize) -> Vec<Variable> {
        let prev = match visibility {
            Visibility::Public => {
                let prev = self.spec[round].0;
                self.spec[round].0 += size;
                prev
            },
            Visibility::Private => {
                let prev = self.spec[round].1;
                self.spec[round].1 += size;
                prev
            },
        };

        (prev..prev+size).into_iter().map(|index| Variable { visibility, round, index }).collect()
    }

    fn constrain(&mut self, kind: CommitKind, inputs: &[Variable], gate: G) {
        self.constraint_group(kind).constrain(inputs, gate);
    }
}
