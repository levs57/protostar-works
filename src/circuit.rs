use std::{rc::{Rc, Weak}, marker::PhantomData, iter::repeat_with, cell::RefCell};
use elsa::map::FrozenMap;
use ff::PrimeField;
use itertools::Itertools;

use crate::{witness::{CSWtns, ProtostarWtns, ProtostarLhsWtns}, gate::{Gatebb, Gate}, constraint_system::{Variable, ProtoGalaxyConstraintSystem, CommitKind, Visibility, CS, Constraint}, utils::poly_utils::check_poly, circuit::circuit_operations::{AttachedAdvice, AttachedPolynomialAdvice, AttachedAdvicePub}, external_interface::{RunIndex, RunAllocator} };

use self::circuit_operations::CircuitOperation;

/// A circuit advice that is guaranteed to be a polynomial function
///
/// Note that while `Gate` expects its output to be zero,
/// this type does not for it would be a pretty boring advice.
#[derive(Clone)]
pub struct PolyOp<'closure, F: PrimeField>{
    pub d: usize,
    pub i: usize,
    pub o: usize,
    pub f: Rc<dyn Fn(&[F], &[F]) -> Vec<F> + 'closure>,
}

impl<'closure, F:PrimeField> PolyOp<'closure, F> {
    pub fn new(d: usize, i: usize, o: usize, f: impl Fn(&[F], &[F]) -> Vec<F> + 'closure) -> Self {
        let f =  Rc::new(f);
        check_poly(d, i, o, f.clone(), &[]).unwrap();

        Self { d, i, o, f }
    }
}

// TODO: impl Gate for PolyOp when CSWitness will support dyn dispatch
impl<'closure, F: PrimeField> From<PolyOp<'closure, F>> for Gatebb<'closure, F>{
    fn from(value: PolyOp<'closure, F>) -> Self {
        // we basically move the rhs (output) to the left
        let d = value.d;
        let i = value.i + value.o;
        let o = value.o;

        let f = move |args: &[F], _: &[F]| {
            let (inputs, outputs) = args.split_at(value.i);
            let results = (value.f)(&inputs, &[]);
            results.iter().zip(outputs.iter()).map(|(res, out)|*res-*out).collect()
        };

        Gatebb::new(d, i, o, Rc::new(f), vec![])   
    }
}

/// An external value used in circuit.
///
/// This can be loaded as a public input or used as an auxiliary value in advices. 
/// Can also be shared between multiple circuits in order to enable layered constructions.
#[derive(Debug, Clone, Copy)]
pub struct ExternalValue<F: PrimeField> {
    pub addr: usize,
    pub _marker: PhantomData<F>,
}

/// An internal value, used by prover but not allocated to witness. 
pub struct InternalValue<F: PrimeField> {
    elt: Option<F>,
}

impl<F: PrimeField> InternalValue<F> {
    pub fn get(&self) -> Option<F> {
        self.elt.clone()
    }
}

/// A (possibly non-polynomial) circuit advice
///
/// Closure inside an advice may depend on some auxiliary values.
 #[derive(Clone)]
 pub struct Advice<'closure, F: PrimeField> {
    pub ivar: usize,
//    pub iext: usize,
    pub o: usize,
    pub f: Rc<dyn Fn(&[F], &RunIndex)-> Vec<F> + 'closure>,
}

impl<'closure, F: PrimeField> Advice<'closure, F> {
    pub fn new(ivar: usize, o: usize, f: impl Fn(&[F], &RunIndex) -> Vec<F> + 'closure) -> Self {
        let f = Rc::new(f);

        Self { ivar, o, f }
    }
}

#[derive(Clone)]
 pub struct AdvicePub<'closure, F: PrimeField> {
    pub iext: usize,
    pub o: usize,
    pub f: Rc<dyn Fn(&[F])-> Vec<F> + 'closure>,
}

impl<'closure, F: PrimeField> AdvicePub<'closure, F> {
    pub fn new(iext: usize, o: usize, f: impl Fn(&[F]) -> Vec<F> + 'closure) -> Self {
        let f = Rc::new(f);

        Self { iext, o, f }
    }
}

pub mod circuit_operations {
    use std::rc::Rc;
    use ff::PrimeField;
    use crate::{constraint_system::Variable, gate::Gate, witness::CSWtns};
    use super::{ExternalValue, RunIndex};

    pub trait CircuitOperation<'a, F: PrimeField, G: Gate<'a, F>> {
        fn execute(&self, witness: &mut CSWtns<'a, F, G>, idx: &RunIndex);
    }

    pub struct AttachedAdvicePub<'advice, F: PrimeField> {
        aux: Vec<ExternalValue<F>>,
        output: Vec<Variable>,
        closure: Rc<dyn Fn(&[F]) -> Vec<F> + 'advice>,
    }

    impl<'advice, F: PrimeField> AttachedAdvicePub<'advice, F> {
        pub fn new(aux: Vec<ExternalValue<F>>, output: Vec<Variable>, closure: Rc<dyn Fn(&[F]) -> Vec<F> + 'advice>) -> Self {
            Self { aux, output, closure }
        }
    }

    impl<'advice, F: PrimeField, G: Gate<'advice, F>> CircuitOperation<'advice, F, G> for AttachedAdvicePub<'advice, F> {
        fn execute(&self, witness: &mut CSWtns<'advice, F, G>, idx: &RunIndex) {
            let aux: Vec<_> = self.aux.iter().map(|ev| witness.getext(*ev)).collect();

            let output = (self.closure)(&aux);

            let value_set: Vec<_> = self.output.iter().cloned().zip(output).collect();
            witness.set_vars(&value_set);
        }
    }


    pub struct AttachedAdvice<'advice, F: PrimeField> {
        input: Vec<Variable>,
        output: Vec<Variable>,
        closure: Rc<dyn Fn(&[F], &RunIndex) -> Vec<F> + 'advice>,
    }

    impl<'advice, F: PrimeField> AttachedAdvice<'advice, F> {
        pub fn new(input: Vec<Variable>, output: Vec<Variable>, closure: Rc<dyn Fn(&[F], &RunIndex) -> Vec<F> + 'advice>) -> Self {
            Self { input, output, closure }
        }
    }

    impl<'advice, F: PrimeField, G: Gate<'advice, F>> CircuitOperation<'advice, F, G> for AttachedAdvice<'advice, F> {
        fn execute(&self, witness: &mut CSWtns<'advice, F, G>, idx: &RunIndex) {
            let input = witness.get_vars(&self.input);

            let output = (self.closure)(&input, &idx);

            let value_set: Vec<_> = self.output.iter().cloned().zip(output).collect();
            witness.set_vars(&value_set);
        }
    }

    pub struct AttachedPolynomialAdvice<'closure, F> {
        input: Vec<Variable>,
        output: Vec<Variable>,
        closure: Rc<dyn Fn(&[F], &[F]) -> Vec<F> + 'closure>,
    }

    impl<'closure, F> AttachedPolynomialAdvice<'closure, F> {
        pub fn new(input: Vec<Variable>, output: Vec<Variable>, closure: Rc<dyn Fn(&[F], &[F]) -> Vec<F> + 'closure>) -> Self {
            Self { input, output, closure }
        }
    }

    impl<'closure, F: PrimeField, G: Gate<'closure, F>> CircuitOperation<'closure, F, G> for AttachedPolynomialAdvice<'closure, F> {
        fn execute(&self, witness: &mut CSWtns<'closure, F, G>, _: &RunIndex) {
            let input = witness.get_vars(&self.input);

            let output = (self.closure)(&input, &[]);

            let value_set: Vec<_> = self.output.iter().cloned().zip(output).collect();
            witness.set_vars(&value_set);
        }
    }
}

pub struct Circuit<'circuit, F: PrimeField, G: Gate<'circuit, F> + From<PolyOp<'circuit, F>>> {
    gate_registry: FrozenMap<String, Box<G>>,
    pub cs: ProtoGalaxyConstraintSystem<'circuit, F, G>,
    ops: Vec<Vec<Box<dyn CircuitOperation<'circuit, F, G> + 'circuit>>>,
    max_degree: usize,
//    round_counter : usize,
//    _state_marker: PhantomData<S>,
}

impl<'circuit, F, G> Circuit<'circuit, F, G>
where
    F: PrimeField,
    G: Gate<'circuit, F> + From<PolyOp<'circuit, F>>,
{
    pub fn new(max_degree: usize, num_rounds: usize) -> Self {
        let cs = ProtoGalaxyConstraintSystem::new(num_rounds);
        let mut prep = Self {
                gate_registry: FrozenMap::new(),
                cs,
                ops: repeat_with(|| Vec::default()).take(num_rounds).collect(),  // this particular Vec::default() is !Clone
                max_degree,
                //_state_marker: PhantomData,
        };

        let load_one = AdvicePub::new( 0, 1, |_| vec![F::ONE]);
        let one = prep.advice_pub(0, load_one, vec![])[0];

        assert!(one == prep.one(),
            "One has allocated incorrectly. This should never fail. Abort mission.");

        prep
    }

    pub fn advice(&mut self, round: usize, advice: Advice<'circuit, F>, input: Vec<Variable>) -> Vec<Variable> {
        assert!(round < self.ops.len(), "The round is too large.");

        let op_index = self.ops[round].len();

        for v in &input {
            assert!(v.round <= round, "Argument {:?} of operation #{} is in round larger than the operation itself ({})", v, op_index, round);
        }

        assert!(input.len() == advice.ivar, "Incorrect amount of input vars at operation #{} (round {})", op_index, round);

        let output = self.cs.alloc_in_round(round, Visibility::Private, advice.o);
        let operation = Box::new(AttachedAdvice::new(input, output.clone(), advice.f.clone()));
        self.ops[round].push(operation);

        output
    }

    pub fn advice_pub(&mut self, round: usize, advice: AdvicePub<'circuit, F>, aux: Vec<ExternalValue<F>>) -> Vec<Variable> {
        assert!(round < self.ops.len(), "The round is too large.");

        let op_index = self.ops[round].len();

        assert!(aux.len() == advice.iext, "Incorrect amount of external vals at operation #{} (round {})", op_index, round);

        let output = self.cs.alloc_in_round(round, Visibility::Public, advice.o);
        let operation = Box::new(AttachedAdvicePub::new(aux, output.clone(), advice.f.clone()));
        self.ops[round].push(operation);

        output
    }

    fn apply_internal(&mut self, visibility: Visibility, round : usize, polyop: PolyOp<'circuit, F>, input: Vec<Variable>) -> Vec<Variable> {
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

    pub fn apply(&mut self, round: usize, polyop: PolyOp<'circuit, F>, input: Vec<Variable>) -> Vec<Variable> {
        self.apply_internal(Visibility::Private, round, polyop, input)
    }

    pub fn apply_pub(&mut self, round : usize, polyop: PolyOp<'circuit, F>, input: Vec<Variable>) -> Vec<Variable> {
        self.apply_internal(Visibility::Public, round, polyop, input)
    }

    // TODO: pass input by value since we clone it down the stack either way
    pub fn constrain(&mut self, input: &[Variable], gate: G) {
        println!("Using legacy unnamed constrains");
        self._constrain(&input, gate)
    }

    fn _constrain(&mut self, input: &[Variable], gate: G) {
        assert!(gate.d() > 0, "Trying to constrain with gate of degree 0.");

        let kind = if gate.d() == 1 { CommitKind::Zero } else { CommitKind::Group };
        self.cs.constrain(kind, input, gate);
    }

    pub fn constrain_with(
        &mut self, 
        input: &[Variable], 
        gate_fetcher: &dyn Fn(&FrozenMap<String, Box<G>>) -> G
    ) {
        let gate = gate_fetcher(&self.gate_registry);
        self._constrain(&input, gate);
    }

    pub fn load_pi(&'circuit mut self, round: usize, pi: ExternalValue<F>) -> Variable {
        let adv = AdvicePub::new(1, 1, move |ext| vec![ext[0]]);
        self.advice_pub(round, adv, vec![pi])[0]
    }

    pub fn finalize(self) -> ConstructedCircuit<'circuit, F, G> {
        ConstructedCircuit {
            circuit: self,
            run_allocator: RefCell::new(RunAllocator::new()),
        }
    }

    pub fn one(&self) -> Variable {
        Variable { visibility: Visibility::Public, round: 0, index: 0 }
    }

    pub fn ext_val(&mut self, size: usize) -> Vec<ExternalValue<F>> {
        self.cs.extval(size)
    }
}

pub struct ConstructedCircuit<'circuit, F: PrimeField, G: Gate<'circuit, F> + From<PolyOp<'circuit, F>>> {
    pub circuit: Circuit<'circuit, F, G>,
    run_allocator: RefCell<RunAllocator>,
}

impl<'circuit, F: PrimeField, G: Gate<'circuit, F> + From<PolyOp<'circuit, F>>> ConstructedCircuit<'circuit, F, G> {
    pub fn spawn<'constructed>(&'constructed self) -> CircuitRun<'constructed, 'circuit, F, G> {
        CircuitRun { 
            constructed: &self, 
            cs: CSWtns::<F,G>::new(&self.circuit.cs), 
            round_counter: 0,
            run_idx: self.run_allocator.borrow_mut().allocate(),
        }
    }

    fn deallocate<'constructed>(&'constructed self, idx: RunIndex) {
        self.run_allocator.borrow_mut().deallocate(idx);
    }

    pub fn perepare_protostar_chellanges(&self, mut beta: F) -> Vec<F> {
        let m = self.circuit.cs.constr_spec().num_nonlinear_constraints;
        let mut p = 1;
        let mut protostar_challenges = vec![];
        while p < m {
            protostar_challenges.push(beta);
            beta = beta * beta;
            p = p * 2;
        }
        protostar_challenges
    }
}

pub struct CircuitRun<'constructed, 'circuit, F: PrimeField, G: Gate<'circuit, F> + From<PolyOp<'circuit, F>>>{
    constructed: &'constructed ConstructedCircuit<'circuit, F, G>,
    pub cs : CSWtns<'circuit, F, G>,
    round_counter : usize,
    run_idx: RunIndex,
}

impl<'constructed, 'circuit, F, G> CircuitRun<'constructed, 'circuit, F, G>
where
    F: PrimeField,
    G: Gate<'circuit, F> + From<PolyOp<'circuit, F>>,
{
    /// Executes the circuit up from the current program counter to round k.
    pub fn execute(&mut self, round: usize) {
        assert!(self.round_counter <= round, "Execution is already at round {}, tried to execute up to round {}", self.round_counter, round);

        while self.round_counter <= round {
            for op in &self.constructed.circuit.ops[self.round_counter] {
                op.execute(&mut self.cs, &self.run_idx);
            }
            self.round_counter += 1;
        }
    }

    pub fn end(&self, beta: F) -> ProtostarWtns<F> {
        let protostar_challenges = self.constructed.perepare_protostar_chellanges(beta);

        let mut pubs = vec![];
        let mut round_wtns = vec![];

        for round in &self.cs.wtns {
            round_wtns.push(round.privs.iter().map(|x| x.unwrap()).collect_vec());
            pubs.push(round.pubs.iter().map(|x| x.unwrap()).collect_vec());
        }

        ProtostarWtns {
            lhs: ProtostarLhsWtns {
                round_wtns,
                pubs,
                protostar_challenges,
            },
            error: F::ZERO,
        }
    }

    pub fn set_ext(&mut self, ext: ExternalValue<F>, value: F) -> () {
        self.cs.setext(ext, value);
    }


    pub fn valid_witness(&self) -> () {
        for constr in self.constructed.circuit.cs.iter_constraints() {
            let input_values: Vec<_> = constr.inputs.iter().map(|&x| self.cs.getvar(x)).collect();
            let result = constr.gate.exec(&input_values);

            assert!(result.iter().all(|&output| output == F::ZERO), "Constraint {:?} is not satisfied", constr);
        }
    }

    pub fn iter_constraints(&self) -> impl Iterator<Item = &Constraint<'circuit, F, G>> {
        self.constructed.circuit.cs.iter_constraints()
    }

    pub fn finish(self) -> (CSWtns<'circuit, F, G>, &'constructed ProtoGalaxyConstraintSystem<'circuit, F, G>) {
        let Self {constructed, cs, round_counter: _, run_idx} = self;

        constructed.deallocate(run_idx);
        (cs, &constructed.circuit.cs)
    }
}

