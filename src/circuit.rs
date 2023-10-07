use std::{rc::Rc, cell::OnceCell, marker::PhantomData};

use ff::PrimeField;

use crate::{witness::CSWtns, gate::{Gatebb, Gate}, constraint_system::{Variable, ConstraintSystem, CommitKind, Visibility, CS}, utils::poly_utils::check_poly};


#[derive(Clone)]
pub struct PolyOp<'a, F:PrimeField>{
    pub d: usize,
    pub i: usize,
    pub o: usize,
    pub f: Rc<dyn Fn(&[F]) -> Vec<F> + 'a>,
}

impl<'a, F:PrimeField> PolyOp<'a, F> {
    pub fn new(d: usize, i: usize, o: usize, f: Rc<dyn Fn(&[F]) -> Vec<F> + 'a>) -> Self{
        check_poly(d, i, o, f.clone()).unwrap(); 
        Self { d, i, o, f }
    }

    pub fn allocate(self, i: Vec<Variable>, o: Vec<Variable>) -> PolyOpAllocated<'a, F> {
        PolyOpAllocated { op : self, i, o }
    }
}

impl<'a, F: PrimeField> From<PolyOp<'a, F>> for Gatebb<'a, F>{
    fn from(value: PolyOp<'a, F>) -> Self {
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

#[derive(Clone)]
pub struct PolyOpAllocated<'a, F: PrimeField> {
    op: PolyOp<'a, F>,
    i: Vec<Variable>,
    o: Vec<Variable>,
}

/// A value used for advices. Can be shared between multiple circuits, in order to enable layered constructions.
pub type ExternalValue<F> = OnceCell<F>;

impl<'a, F: PrimeField> Advice<'a, F> {
    pub fn new(ivar: usize, iext:usize, o: usize, f: Rc<dyn Fn(&[F], &[F]) -> Vec<F> + 'a>) -> Self{
        Self{ ivar, iext, o, f }
    }

    pub fn allocate(self: Advice<'a, F>, ivar: Vec<Variable>, iext: Vec<&'a ExternalValue<F>>, o: Vec<Variable>) -> AdviceAllocated<'a, F> {
        AdviceAllocated { adv : self , ivar, iext, o }
    }
 }

 #[derive(Clone)]
 pub struct Advice<'a, F: PrimeField> {
    pub ivar: usize,
    pub iext: usize,
    pub o: usize,
    pub f: Rc<dyn Fn(&[F], &[F])-> Vec<F> + 'a>,
}

#[derive(Clone)]
pub struct AdviceAllocated<'a, F: PrimeField> {
    adv: Advice<'a, F>,
    ivar: Vec<Variable>,
    iext: Vec<&'a ExternalValue<F>>,
    o: Vec<Variable>,
 }

 #[derive(Clone)]
pub enum Operation<'a, F: PrimeField> {
    Poly(PolyOpAllocated<'a, F>),
    Adv(AdviceAllocated<'a, F>),
}

pub struct Build;
pub struct Finalized;

mod private {
    use super::{Build, Finalized};

    pub trait CircuitState {}

    impl CircuitState for Build {}
    impl CircuitState for Finalized {}
}

#[derive(Clone)]
pub struct Circuit<'a, F: PrimeField, T:Gate<F> + From<PolyOp<'a, F>>, S: private::CircuitState> {
    pub cs: CSWtns<F, T>,
    pub ops: Vec<Vec<Operation<'a, F>>>,
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
                ops: vec![vec![]; num_rounds],
                max_degree,
                round_counter: 0,
                _state_marker: PhantomData,
        };

        let adv = Advice::new(0, 0, 1, Rc::new(|_, _| vec![F::ONE]));
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
        self.ops[round].push(Operation::Adv(advice.allocate(input, aux, output.clone())));

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
        self.ops[round].push(Operation::Poly(polyop.clone().allocate(input.clone(), output.clone())));

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
        let adv = Advice::new(0, 1, 1, Rc::new(move |_, ext| vec![ext[0]]));
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
            for op in &self.ops[self.round_counter] {
                match op {
                    Operation::Poly(polyop) => {
                        let input: Vec<_> = polyop.i.iter().map(|&v| self.cs.getvar(v)).collect();

                        let output = (polyop.op.f)(&input);

                        polyop.o.iter().zip(output.iter()).for_each(|(&var, &value)| self.cs.setvar(var, value));
                    }
                    Operation::Adv(adv) => {
                        let input: Vec<_> = adv.ivar.iter().map(|&v| self.cs.getvar(v)).collect();
                        let aux: Vec<_> = adv.iext.iter().map(|&x| *x.get().unwrap()).collect();

                        let output = (adv.adv.f)(&input, &aux);

                        adv.o.iter().zip(output.iter()).for_each(|(&var, &value)| self.cs.setvar(var, value));
                    }
                }
            }
            self.round_counter += 1;
        }
    }
}