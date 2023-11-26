use std::{marker::PhantomData, collections::BTreeMap};

use ff::PrimeField;
use itertools::Itertools;

use crate::{gate::Gate, circuit::ExternalValue};

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
pub struct Constraint<'c, F: PrimeField, G: Gate<'c, F>>{
    pub inputs: Vec<Variable>,
    pub gate: G,
    _marker: PhantomData<&'c F>,
}

/// Constraints are grouped by their CommitKind.
/// 
/// Currently this struct has some additional information. This will probably
/// be moved in the near future
#[derive(Debug, Clone)]
struct ConstraintGroup<'c, F: PrimeField, G: Gate<'c, F>> {
    pub entries: Vec<Constraint<'c, F, G>>,
    pub num_rhs: usize,
}

impl<'c, F: PrimeField, G: Gate<'c, F>> ConstraintGroup<'c, F, G> {
    pub fn new() -> Self {
        Self {
            entries: Default::default(),
            num_rhs: Default::default(),
        }
    }

    pub fn constrain(&mut self, inputs: &[Variable], gate: G) {
        assert!(gate.i() == inputs.len(), "Invalid amount of arguments supplied.");

        self.num_rhs += gate.o();
        self.entries.push(Constraint{inputs : inputs.to_vec(), gate, _marker : PhantomData});
    }
}

/// Round witness shape specification: the amount of public and private variables respectively
/// 
/// Any witness used for this constraint system has to at least comply with the spec.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct RoundWitnessSpec{
    pub pubs: usize,
    pub privs: usize,
}

/// Witness shape specification: a collection of specifications for each round
/// 
/// Any witness used for this constraint system has to at least comply with the spec.
#[derive(Debug, Clone)]
pub struct WitnessSpec {
    pub round_specs: Vec<RoundWitnessSpec>,
    pub num_ints: usize,
    pub num_exts: usize,
}

#[derive(Debug, Clone)]
pub struct ConstrSpec {
    pub num_lin_constraints: usize,
    pub num_nonlinear_constraints: usize,
    pub max_degree: usize,
}

pub trait CS<'c, F: PrimeField, G: Gate<'c, F>> {    
    fn num_rounds(&self) -> usize;

    fn last_round(&self) -> usize {
        self.num_rounds() - 1
    }

    fn new_round(&mut self);

    fn constr_spec(&self) -> ConstrSpec;

    fn witness_spec(&self) -> &WitnessSpec;

    fn alloc_in_round(&mut self, round: usize, visibility: Visibility, size: usize) -> Vec<Variable>;

    fn alloc(&mut self, visibility: Visibility, size: usize) -> Vec<Variable> {
        self.alloc_in_round(self.last_round(), visibility, size)
    }

    fn constrain(&mut self, kind: CommitKind, inputs: &[Variable], gate: G);

    fn extval(&mut self, size: usize) -> Vec<ExternalValue<F>>; 
}

/// This CS is made specifically for ProtoGalaxy.
/// We will probably make interface for this part 
#[derive(Debug, Clone)]
pub struct ProtoGalaxyConstraintSystem<'c, F: PrimeField, G: Gate<'c, F>> {
    pub spec: WitnessSpec,
    pub max_degree: usize,
    linear_constraints: ConstraintGroup<'c, F, G>,
    non_linear_constraints: BTreeMap<usize, ConstraintGroup<'c, F, G>>,
}

impl<'c, F: PrimeField, G: Gate<'c, F>> ProtoGalaxyConstraintSystem<'c, F, G> {
    pub fn new(num_rounds: usize) -> Self {
        Self {
            spec: WitnessSpec{ round_specs: vec![RoundWitnessSpec::default(); num_rounds], num_exts: 0, num_ints: 0 },
            max_degree: 0,
            linear_constraints: ConstraintGroup::new(),
            non_linear_constraints: BTreeMap::new(),
        }
    }

    // would love to add this to the trait, but crab god said not yet
    // https://github.com/rust-lang/rust/issues/91611
    pub fn iter_constraints(&self) -> impl Iterator<Item = &Constraint<'c, F, G>> {
        self.iter_linear_constraints().chain(self.iter_non_linear_constraints())
    }

    pub fn iter_linear_constraints(&self) -> impl Iterator<Item = &Constraint<'c, F, G>> {
        self.linear_constraints.entries.iter()
    }

    pub fn iter_non_linear_constraints(&self) -> impl Iterator<Item = &Constraint<'c, F, G>> {
        self.non_linear_constraints.iter().flat_map(|(_, cg)| cg.entries.iter())
    }
}

impl<'c, F: PrimeField, G: Gate<'c, F>> CS<'c, F, G> for ProtoGalaxyConstraintSystem<'c, F, G> {    
    fn num_rounds(&self) -> usize {
        self.spec.round_specs.len()
    }

    fn new_round(&mut self) {
        self.spec.round_specs.push(RoundWitnessSpec::default())
    }

    fn witness_spec(&self) -> &WitnessSpec {
        &self.spec
    }

    fn constr_spec(&self) -> ConstrSpec {
        let num_lin_constraints = self.linear_constraints.num_rhs;
        let num_nonlinear_constraints = self.non_linear_constraints.iter().map(|(_, cg)| cg.num_rhs).sum();
        let max_degree = self.max_degree;
        ConstrSpec { num_lin_constraints, num_nonlinear_constraints, max_degree }
    }

    fn alloc_in_round(&mut self, round: usize, visibility: Visibility, size: usize) -> Vec<Variable> {
        let prev = match visibility {
            Visibility::Public => {
                let prev = self.spec.round_specs[round].pubs;
                self.spec.round_specs[round].pubs += size;
                prev
            },
            Visibility::Private => {
                let prev = self.spec.round_specs[round].privs;
                self.spec.round_specs[round].privs += size;
                prev
            },
        };

        (prev..prev+size).into_iter().map(|index| Variable { visibility, round, index }).collect()
    }

    fn constrain(&mut self, _: CommitKind, inputs: &[Variable], gate: G) {
        self.max_degree = self.max_degree.max(gate.d());
        match gate.d().cmp(&1) {
            std::cmp::Ordering::Less => panic!("Constraint of degree 0"),
            std::cmp::Ordering::Equal => {
                self.linear_constraints.constrain(inputs, gate)
            },
            std::cmp::Ordering::Greater => {
                self.non_linear_constraints.entry(gate.d()).or_insert(ConstraintGroup::new()).constrain(inputs, gate)
            },
        }
    }

    fn extval(&mut self, size: usize) -> Vec<ExternalValue<F>> {
        let prev = self.spec.num_exts;
        self.spec.num_exts += size;
        (prev..prev+size).into_iter().map(|x|ExternalValue{addr:x, _marker: PhantomData::<F>}).collect()
    }
}
