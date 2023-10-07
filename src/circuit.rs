use std::{rc::Rc, cell::OnceCell, marker::PhantomData, iter::repeat_with};

use ff::PrimeField;

use crate::{witness::CSWtns, gate::{Gatebb, Gate}, constraint_system::{Variable, ConstraintSystem, CommitKind, Visibility, CS}, utils::poly_utils::check_poly, circuit::circuit_operations::{AttachedAdvice, AttachedPolynomialAdvice}};

use self::circuit_operations::CircuitOperation;

/// A circuit advice that is guaranteed to be a polynomial function
///
/// Note that while `Gate` expects its output to be zero,
/// this type does not for it would be a pretty boring advice.
#[derive(Clone)]
pub struct PolyOp<'a, F:PrimeField>{
    pub d: usize,
    pub i: usize,
    pub o: usize,
    pub f: Rc<dyn Fn(&[F]) -> Vec<F> + 'a>,
}

impl<'a, F:PrimeField> PolyOp<'a, F> {
    pub fn new(d: usize, i: usize, o: usize, f: impl Fn(&[F]) -> Vec<F> + 'a) -> Self {
        let f =  Rc::new(f);
        check_poly(d, i, o, f.clone()).unwrap();

        Self { d, i, o, f }
    }
}

impl<'a, F: PrimeField> From<PolyOp<'a, F>> for Gatebb<'a, F>{
    fn from(value: PolyOp<'a, F>) -> Self {
        // we basically move the rhs (output) to the left
        let d = value.d;
        let i = value.i + value.o;
        let o = value.o;

        let f = move |args: &[F]| {
            let (inputs, outputs) = args.split_at(value.i);
            let results = (value.f)(&inputs);
            results.iter().zip(outputs.iter()).map(|(res, out)|*res-*out).collect()
        };

        Gatebb::new(d, i, o, Rc::new(f))   
    }
}

/// An external value used in circuit.
///
/// This can be loaded as a public input or used as an auxiliary value in advices. 
/// Can also be shared between multiple circuits in order to enable layered constructions.
pub type ExternalValue<F> = OnceCell<F>;

/// A (possibly non-polynomial) circuit advice
///
/// Closure inside an advice may depend on some auxiliary values.
 #[derive(Clone)]
 pub struct Advice<'a, F: PrimeField> {
    pub ivar: usize,
    pub iext: usize,
    pub o: usize,
    pub f: Rc<dyn Fn(&[F], &[F])-> Vec<F> + 'a>,
}

impl<'a, F: PrimeField> Advice<'a, F> {
    pub fn new(ivar: usize, iext: usize, o: usize, f: impl Fn(&[F], &[F]) -> Vec<F> + 'a) -> Self {
        let f = Rc::new(f);

        Self { ivar, iext, o, f }
    }
}

pub mod circuit_operations {
    use std::rc::Rc;

    use ff::PrimeField;

    use crate::{constraint_system::Variable, gate::Gate, witness::CSWtns};

    use super::ExternalValue;

    pub trait CircuitOperation<F: PrimeField, G: Gate<F>> {
        fn execute(&self, witness: &mut CSWtns<F, G>);
    }

    pub struct AttachedAdvice<'circuit, F> {
        input: Vec<Variable>,
        aux: Vec<&'circuit ExternalValue<F>>,
        output: Vec<Variable>,
        closure: Rc<dyn Fn(&[F], &[F]) -> Vec<F> + 'circuit>,
    }

    impl<'circuit, F> AttachedAdvice<'circuit, F> {
        pub fn new(input: Vec<Variable>, aux: Vec<&'circuit ExternalValue<F>>, output: Vec<Variable>, closure: Rc<dyn Fn(&[F], &[F]) -> Vec<F> + 'circuit>) -> Self {
            Self { input, aux, output, closure }
        }
    }

    impl<'circuit, F: PrimeField, G: Gate<F>> CircuitOperation<F, G> for AttachedAdvice<'circuit, F> {
        fn execute(&self, witness: &mut CSWtns<F, G>) {
            let input = witness.get_vars(&self.input);
            let aux: Vec<_> = self.aux.iter().map(|ev| *ev.get().expect("external values should be set before execution")).collect();

            let output = (self.closure)(&input, &aux);

            let value_set: Vec<_> = self.output.iter().cloned().zip(output).collect();
            witness.set_vars(&value_set);
        }
    }

    pub struct AttachedPolynomialAdvice<'circuit, F> {
        input: Vec<Variable>,
        output: Vec<Variable>,
        closure: Rc<dyn Fn(&[F]) -> Vec<F> + 'circuit>,
    }

    impl<'circuit, F> AttachedPolynomialAdvice<'circuit, F> {
        pub fn new(input: Vec<Variable>, output: Vec<Variable>, closure: Rc<dyn Fn(&[F]) -> Vec<F> + 'circuit>) -> Self {
            Self { input, output, closure }
        }
    }

    impl<'circuit, F: PrimeField, G: Gate<F>> CircuitOperation<F, G> for AttachedPolynomialAdvice<'circuit, F> {
        fn execute(&self, witness: &mut CSWtns<F, G>) {
            let input = witness.get_vars(&self.input);

            let output = (self.closure)(&input);

            let value_set: Vec<_> = self.output.iter().cloned().zip(output).collect();
            witness.set_vars(&value_set);
        }
    }
}


pub struct Build;
pub struct Finalized;

mod private {
    use super::{Build, Finalized};

    pub trait CircuitState {}

    impl CircuitState for Build {}
    impl CircuitState for Finalized {}
}

pub struct Circuit<'circuit, F: PrimeField, T: Gate<F> + From<PolyOp<'circuit, F>>, S: private::CircuitState> {
    pub cs: CSWtns<F, T>,
    pub ops: Vec<Vec<Box<dyn CircuitOperation<F, T> + 'circuit>>>,
    pub max_degree: usize,
    pub round_counter : usize,
    _state_marker: PhantomData<S>,
}

impl<'a, F, T> Circuit<'a, F, T, Build>
where
    F: PrimeField,
    T: Gate<F> + From<PolyOp<'a, F>>,
{
    pub fn new(max_degree: usize, num_rounds: usize) -> Self {
        let cs = ConstraintSystem::new(num_rounds, max_degree);
        let mut prep = Self {
                cs : CSWtns::new(cs),
                ops: repeat_with(|| Vec::default()).take(num_rounds).collect(),  // this particular Vec::default() is !Clone
                max_degree,
                round_counter: 0,
                _state_marker: PhantomData,
        };

        let adv = Advice::new(0, 0, 1, |_, _| vec![F::ONE]);
        let tmp = prep.advice_pub(0, adv, vec![], vec![]);

        assert!(tmp[0] == prep.one(),
            "One has allocated incorrectly. This should never fail. Abort mission.");

        prep
    }

    pub fn advice_internal(&mut self, visibility: Visibility, round: usize, advice: Advice<'a, F>, input: Vec<Variable>, aux: Vec<&'a ExternalValue<F>>) -> Vec<Variable> {
        assert!(round < self.ops.len(), "The round is too large.");

        let op_index = self.ops[round].len();

        for v in &input {
            assert!(v.round <= round, "Argument {:?} of operation #{} is in round larger than the operation itself ({})", v, op_index, round);
        }

        assert!(input.len() == advice.ivar, "Incorrect amount of input vars at operation #{} (round {})", op_index, round);
        assert!(aux.len() == advice.iext, "Incorrect amount of external vals at operation #{} (round {})", op_index, round);

        let output = self.cs.alloc_in_round(round, visibility, advice.o);
        let operation = Box::new(AttachedAdvice::new(input, aux, output.clone(), advice.f.clone()));
        self.ops[round].push(operation);

        output
    }

    pub fn advice(&mut self, round: usize, advice: Advice<'a, F>, input: Vec<Variable>, aux: Vec<&'a ExternalValue<F>>) -> Vec<Variable> {
        self.advice_internal(Visibility::Private, round, advice, input, aux)
    }

    pub fn advice_pub(&mut self, round: usize, advice: Advice<'a, F>, input: Vec<Variable>, aux: Vec<&'a ExternalValue<F>>) -> Vec<Variable> {
        self.advice_internal(Visibility::Public, round, advice, input, aux)
    }

    fn apply_internal(&mut self, visibility: Visibility, round : usize, polyop: PolyOp<'a, F>, input: Vec<Variable>) -> Vec<Variable> {
        assert!(round < self.ops.len(), "The round is too large.");

        let op_index = self.ops[round].len();

        for v in &input {
            assert!(v.round <= round, "Argument {:?} of operation #{} is in round larger than the operation itself ({})", v, op_index, round);
        }

        assert!(polyop.d > 0, "Operation #{} has degree 0", op_index);
        assert!(polyop.d <= self.max_degree, "Degree of operation #{} is too large", op_index);

        assert!(input.len() == polyop.i, "Incorrect amount of inputs at operation #{} (round {})", op_index, round);

        let output = self.cs.alloc_in_round(round, visibility, polyop.o);
        let operation = Box::new(AttachedPolynomialAdvice::new(input.clone(), output.clone(), polyop.f.clone()));
        self.ops[round].push(operation);

        let mut gate_io = input;  // do not move input into new buffer
        gate_io.extend(output.iter().cloned());

        self.constrain(&gate_io, polyop.into());
        
        output
    }

    pub fn apply(&mut self, round: usize, polyop: PolyOp<'a, F>, input: Vec<Variable>) -> Vec<Variable> {
        self.apply_internal(Visibility::Private, round, polyop, input)
    }

    pub fn apply_pub(&mut self, round : usize, polyop: PolyOp<'a, F>, input: Vec<Variable>) -> Vec<Variable> {
        self.apply_internal(Visibility::Public, round, polyop, input)
    }

    // TODO: pass input by value since we clone it down the stack either way
    pub fn constrain(&mut self, input: &[Variable], gate: T) {
        assert!(gate.d() > 0, "Trying to constrain with gate of degree 0.");

        let kind = if gate.d() == 1 { CommitKind::Zero } else { CommitKind::Group };
        self.cs.cs.constrain(kind, input, gate);
    }

    pub fn load_pi(&'a mut self, round: usize, pi: &'a ExternalValue<F>) -> Variable {
        let adv = Advice::new(0, 1, 1, move |_, ext| vec![ext[0]]);
        self.advice_pub(round, adv, vec![], vec![&pi])[0]
    }

    pub fn finalize(self) -> Circuit<'a, F, T, Finalized> {
        Circuit {
            cs: self.cs,
            ops: self.ops,
            max_degree: self.max_degree,
            round_counter: self.round_counter,
            _state_marker: PhantomData,
        }
    }

    pub fn one(&self) -> Variable {
        Variable { visibility: Visibility::Public, round: 0, index: 0 }
    }
}

impl<'a, F, T> Circuit<'a, F, T, Finalized>
where
    F: PrimeField,
    T: Gate<F> + From<PolyOp<'a, F>>,
{
    /// Executes the circuit up from the current program counter to round k.
    pub fn execute(&mut self, round: usize) {
        assert!(self.round_counter <= round, "Execution is already at round {}, tried to execute up to round {}", self.round_counter, round);

        while self.round_counter <= round {
            for op in self.ops[self.round_counter].iter() {
                op.execute(&mut self.cs);
            }
            self.round_counter += 1;
        }
    }
}