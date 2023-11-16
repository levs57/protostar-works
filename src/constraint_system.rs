use std::{marker::PhantomData, ops::{Index, IndexMut}, iter::repeat};

use ff::PrimeField;

use crate::{gate::Gate, circuit::ExternalValue};

/// Constraint commitment kind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommitKind {
    Trivial,
    Group,
    Zero, // Used in cases where we do not need to commit.
}

/// Variable descriptor. We treat challenges as public variables
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    Public,
    Private,
}

/// A variable inside a constraint system.
/// 
/// Variables are what constraints operate on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub constants: Vec<F>,
    pub gate: G,
    _marker: PhantomData<&'c F>,
}

// impl<'c, F: PrimeField, G: Gate<'c, F>> Constraint<'c, F, G> {
//     pub fn is_satisfied(&self, wtns: &CSWtns<'c, F, G>) -> bool {
//         let input_values: Vec<_> = self.inputs.iter().map(|&x| wtns.getvar(x)).collect();
//         let result = self.gate.exec(&input_values);

//         result.iter().all(|&output| output == F::ZERO)
//     }
// }

#[derive(Debug, Default, Clone)]
pub struct VariableMetadata {
    pub max_constraint_degree: usize,
}

impl VariableMetadata {
    pub fn update_mcd(&mut self, new_degree: usize) {
        if new_degree > self.max_constraint_degree {
            self.max_constraint_degree = new_degree;
        }
    }
}

type Store = Vec<Vec<VariableMetadata>>;

/// An environment stores metadata of each variable
#[derive(Debug, Default, Clone)]
pub struct Environment {
    pubs: Store,
    privs: Store,
}

impl Environment {
    pub fn new(num_rounds: usize) -> Self {
        Self {
            pubs: vec![Vec::default(); num_rounds],
            privs: vec![Vec::default(); num_rounds],
        }
    }
}

impl Index<Variable> for Environment {
    type Output = VariableMetadata;

    fn index(&self, index: Variable) -> &Self::Output {
        let store = match index.visibility {
            Visibility::Public => &self.pubs,
            Visibility::Private => &self.privs,
        };

        &store[index.round][index.index]
    }
}

impl IndexMut<Variable> for Environment {
    fn index_mut(&mut self, index: Variable) -> &mut Self::Output {
        let store = match index.visibility {
            Visibility::Public => &mut self.pubs,
            Visibility::Private => &mut self.privs,
        };

        &mut store[index.round][index.index]
    }
}


/// Constraints are grouped by their CommitKind.
/// 
/// Currently this struct has some additional information. This will probably
/// be moved in the near future
#[derive(Debug, Clone)]
struct ConstraintGroup<'c, F: PrimeField, G: Gate<'c, F>>{
    pub entries: Vec<Constraint<'c, F, G>>,
    pub kind: CommitKind,
    pub num_rhs: usize,
    pub max_degree: usize,
}

impl<'c, F: PrimeField, G: Gate<'c, F>> ConstraintGroup<'c, F, G> {
    pub fn new(kind: CommitKind, max_degree: usize) -> Self {
        Self {
            entries: Default::default(),
            kind,
            num_rhs: Default::default(),
            max_degree,
        }
    }

    pub fn constrain(&mut self, inputs: &[Variable], constants: &[F], gate: G) {
        assert!(gate.d() <= self.max_degree, "Constraint degree is too large for this group.");
        assert!(gate.i() == inputs.len(), "Invalid amount of arguments supplied.");

        self.num_rhs += gate.o();
        self.entries.push(Constraint{inputs : inputs.to_vec(), constants: constants.to_vec(), gate, _marker : PhantomData});
    }
}

/// Round witness shape specification: the amount of public and private variables respectively
/// 
/// Any witness used for this constraint system has to at least comply with the spec.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct RoundWitnessSpec(pub usize, pub usize);

/// Witness shape specification: a collection of specifications for each round
/// 
/// Any witness used for this constraint system has to at least comply with the spec.
#[derive(Debug, Clone)]
pub struct WitnessSpec {
    pub round_specs: Vec<RoundWitnessSpec>,
    pub num_ints: usize,
    pub num_exts: usize,
}

pub trait CS<'c, F: PrimeField, G: Gate<'c, F>> {
    fn num_rounds(&self) -> usize;

    fn last_round(&self) -> usize {
        self.num_rounds() - 1
    }

    fn new_round(&mut self) -> usize;

    fn witness_spec(&self) -> &WitnessSpec;

    fn alloc_in_round(&mut self, round: usize, visibility: Visibility, size: usize) -> Vec<Variable>;

    fn alloc(&mut self, visibility: Visibility, size: usize) -> Vec<Variable> {
        self.alloc_in_round(self.last_round(), visibility, size)
    }

    fn constrain(&mut self, kind: CommitKind, inputs: &[Variable], constnts: &[F], gate: G);

    fn extval(&mut self, size: usize) -> Vec<ExternalValue<F>>; 
}

#[derive(Debug)]
pub struct ConstraintSystem<'c, F: PrimeField, G: Gate<'c, F>> {
    spec: WitnessSpec,
    constraint_groups: [ConstraintGroup<'c, F, G>; 3],
    pub env: Environment,
}

impl<'c, F: PrimeField, G: Gate<'c, F>> ConstraintSystem<'c, F, G> {
    pub fn new(num_rounds: usize, max_degree: usize) -> Self {
        let constraint_groups = [
            ConstraintGroup::new(CommitKind::Trivial, 0),  // FIXME: correct max_degree
            ConstraintGroup::new(CommitKind::Group, max_degree),
            ConstraintGroup::new(CommitKind::Zero, 1),
        ];

        Self {
            spec: WitnessSpec{ round_specs: vec![RoundWitnessSpec::default(); num_rounds], num_exts: 0, num_ints: 0 },
            env: Environment::new(num_rounds),
            constraint_groups,
        }
    }

    /// A (short-lived) cursor to the constraint group of a given kind
    fn constraint_group(&mut self, kind: CommitKind) -> &mut ConstraintGroup<'c, F, G> {
        match kind {
            CommitKind::Trivial => &mut self.constraint_groups[0],
            CommitKind::Group => &mut self.constraint_groups[1],
            CommitKind::Zero => &mut self.constraint_groups[2],
        }
    }

    // would love to add this to the trait, but crab god said not yet
    // https://github.com/rust-lang/rust/issues/91611
    pub fn iter_constraints(&self) -> impl Iterator<Item = &Constraint<'c, F, G>> {
        self.constraint_groups.iter().flat_map(|cg| cg.entries.iter())
    }

    pub fn iter_variables(&self) -> impl Iterator<Item = Variable> {
        let spec = self.witness_spec().clone();

        spec.round_specs.into_iter()
            .enumerate()
            .map(|(round, RoundWitnessSpec(n_pubs, n_privs))| {
                let pubs = (0..n_pubs).map(move |index| Variable { visibility: Visibility::Public, round, index });
                let privs = (0..n_privs).map(move |index| Variable { visibility: Visibility::Private, round, index });

                pubs.chain(privs)
            })
            .flatten()
    }
}

impl<'c, F: PrimeField, G: Gate<'c, F>> CS<'c, F, G> for ConstraintSystem<'c, F, G> {
    fn num_rounds(&self) -> usize {
        self.spec.round_specs.len()
    }

    fn new_round(&mut self) -> usize {
        self.spec.round_specs.push(RoundWitnessSpec::default());
        
        self.env.pubs.push(Vec::default());
        self.env.privs.push(Vec::default());

        self.last_round()
    }

    fn witness_spec(&self) -> &WitnessSpec {
        &self.spec
    }

    fn alloc_in_round(&mut self, round: usize, visibility: Visibility, size: usize) -> Vec<Variable> {
        let prev = match visibility {
            Visibility::Public => {
                let prev = self.spec.round_specs[round].0;
                self.spec.round_specs[round].0 += size;
                self.env.pubs[round].extend(repeat(VariableMetadata::default()).take(size));
                prev
            },
            Visibility::Private => {
                let prev = self.spec.round_specs[round].1;
                self.spec.round_specs[round].1 += size;
                self.env.privs[round].extend(repeat(VariableMetadata::default()).take(size));
                prev
            },
        };

        (prev..prev+size).into_iter().map(|index| Variable { visibility, round, index }).collect()
    }

    fn extval(&mut self, size: usize) -> Vec<ExternalValue<F>> {
        let prev = self.spec.num_exts;
        self.spec.num_exts += size;
        (prev..prev+size).into_iter().map(|x|ExternalValue{addr:x, _marker: PhantomData::<F>}).collect()
    }

    fn constrain(&mut self, kind: CommitKind, inputs: &[Variable], constants: &[F], gate: G) {
        // update max constraint degree if necessary
        for &var in inputs {
            self.env[var].update_mcd(gate.d())
        }

        self.constraint_group(kind).constrain(inputs, constants, gate);
    }
}