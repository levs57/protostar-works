use std::{iter::repeat, rc::Rc, cell::{Cell, OnceCell}};

use ff::PrimeField;
use num_traits::pow;
use rand_core::OsRng;

use crate::{witness::CSWtns, gate::{Gatebb, RootsOfUnity, Gate}, constraint_system::{Variable, ConstraintSystem, CommitKind}};


#[derive(Clone)]
pub struct PolyOp<'a, F:PrimeField>{
    pub d: usize,
    pub i: usize,
    pub o: usize,
    pub f: Rc<dyn Fn(F, &[F]) -> Vec<F> + 'a>,
}

impl<'a, F:PrimeField> PolyOp<'a, F> {
    pub fn new(d: usize, i: usize, o: usize, f: Rc<dyn Fn(F, &[F]) -> Vec<F> + 'a>) -> Self{
        let random_input : (_, Vec<_>) = (F::random(OsRng) , repeat(F::random(OsRng)).take(i).collect()); 
        let random_input_2 : (_, Vec<_>) = (random_input.0*F::from(2), random_input.1.iter().map(|x| *x*F::from(2)).collect());
        assert!({
            let mut flag = true;
            (&f)(random_input_2.0, &random_input_2.1).iter().zip((&f)(random_input.0, &random_input.1).iter())
                .map(|(a, b)| {
                    flag &= (*a==*b*F::from(pow(2, d)))
                }).count();
            flag
        }, "Sanity check failed - provided f is not a polynomial of degree d");
 
        Self { d, i, o, f }
    }

    pub fn allocate(self, i: Vec<Variable>, o: Vec<Variable>) -> PolyOpAllocated<'a, F> {
        PolyOpAllocated { op : self, i, o }
    }
}

impl<'a, F: PrimeField> From<PolyOp<'a, F>> for Gatebb<'a, F>{
    fn from(value: PolyOp<'a, F>) -> Self {
        let d = value.d;
        let i = value.i + 1 + value.o;
        let o = value.o;

        let f = move |args: &[F]| {
            let (inputs, outputs) = args.split_at(value.i+1);
            let (one, inputs) = inputs.split_at(1);
            let one = one[0];
            let results = (value.f)(one, &inputs);
            let onepow = one.pow([(value.d-1) as u64]);
            results.iter().zip(outputs.iter()).map(|(inp, out)|*inp-*out*onepow).collect()
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
pub type ExternalValue<F: PrimeField> = OnceCell<F>;

// impl<F: PrimeField> ExternalValue<F> {
//     pub fn new() -> Self {
//         Self { v : Cell::new(MaybeValue::new())}
//     }

//     pub fn get(&self) -> F {
//         let tmp = self.v.get();
//         if tmp.flag { 
//             tmp.value
//         } else {
//             panic!("Unassigned external value error.")
//         }
//     }

//     pub fn set(&self, value: F) -> () {
//         let tmp = self.v.get();
//         if tmp.flag {panic!("Can not assign external value twice.")}
//         self.v.set(MaybeValue {value, flag:true })
//     }
// }

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

#[derive(Clone)]
pub struct Circuit<'a, F: PrimeField, T:Gate<F> + From<PolyOp<'a, F>>> {
    pub cs: CSWtns<F, T>,
    pub ops: Vec<Vec<Operation<'a, F>>>,
    pub max_degree: usize,
    pub finalized: bool,
    pub round_counter : usize,
}

impl<'a, F: PrimeField + RootsOfUnity, T: Gate<F> + From<PolyOp<'a, F>>> Circuit<'a, F, T>{
    pub fn new(max_degree: usize, num_rounds: usize) -> Self {
        let mut cs = ConstraintSystem::new(num_rounds);
        cs.add_constr_group(CommitKind::Zero, 1);
        cs.add_constr_group(CommitKind::Group, max_degree);
        let mut prep = Self{
                cs : CSWtns::new(cs),
                ops: vec![vec![];num_rounds],
                max_degree,
                finalized: false,
                round_counter : 0,
            };
        let adv = Advice::new(0,0,1, Rc::new(|_,_|vec![F::ONE]));
        let tmp = prep.advice_pub(0, adv, vec![], vec![]);
        match tmp[0] {
            Variable::Public(0,0) => (),
            _ => panic!("One has allocated incorrectly. This should never fail. Abort mission."),
        };
        prep
    }

    pub fn advice(&mut self, round: usize, adv: Advice<'a, F>, ivar: Vec<Variable>, iext: Vec<&'a ExternalValue<F>>) -> Vec<Variable> {
        assert!(!self.finalized, "Circuit is already built.");
        assert!(round < self.ops.len(), "The round is too large.",);
        assert!(ivar.len() == adv.ivar, "Incorrect amount of inputs at operation {} ; {}", round, self.ops[round].len());
        assert!(iext.len() == adv.iext, "Incorrect amount of advice inputs at operation {} ; {}", round, self.ops[round].len());
        for v in &ivar {
            assert!(v.round() <= round, "Argument of an operation {} ; {} is in round {}", round, self.ops[round].len(), v.round())
        }

        let mut output = vec![];
        for _ in 0..adv.o {
            output.push( self.cs.alloc_priv_internal(round) );
        }
        self.ops[round].push(Operation::Adv(adv.allocate(ivar, iext, output.clone())));
        output
    }

    pub fn advice_pub(&mut self, round: usize, adv: Advice<'a, F>, ivar: Vec<Variable>, iext: Vec<&'a ExternalValue<F>>) -> Vec<Variable> {
        assert!(round < self.ops.len(), "The round is too large.",);
        assert!(ivar.len() == adv.ivar, "Incorrect amount of inputs at operation {} ; {}", round, self.ops[round].len());
        assert!(iext.len() == adv.iext, "Incorrect amount of advice inputs at operation {} ; {}", round, self.ops[round].len());
        for v in &ivar {
            assert!(v.round() <= round, "Argument of an operation {} ; {} is in round {}", round, self.ops[round].len(), v.round())
        }
        let mut output = vec![];
        for _ in 0..adv.o {
            output.push( self.cs.alloc_pub_internal(round) );
        }
        self.ops[round].push(Operation::Adv(adv.allocate(ivar, iext, output.clone())));
        output
    }

    pub fn apply(&mut self, round: usize, polyop: PolyOp<'a, F>, i: Vec<Variable>) -> Vec<Variable> {
        assert!(!self.finalized, "Circuit is already built.");
        assert!(round < self.ops.len(), "The round is too large.",);
        assert!(i.len() == polyop.i, "Incorrect amount of inputs at operation {}", self.ops.len());
        for v in &i {
            assert!(v.round() <= round, "Argument of an operation {} ; {} is in round {}", round, self.ops[round].len(), v.round())
        }
        let mut output = vec![];
        for _ in 0..polyop.o {
            output.push( self.cs.alloc_priv_internal(round) );
        }

        let mut gate_io = vec![Variable::Public(0,0)];
        gate_io.append(&mut i.clone());
        gate_io.append(&mut output.clone());
        if polyop.d == 0 {panic!("Operation {} has degree 0, which is banned.", self.ops.len())}
        if polyop.d > self.max_degree {panic!("Degree of operation {} is too large!", self.ops.len())};
        if polyop.d == 1 {
            self.cs.cs.cs[0].constrain(&gate_io, T::from(polyop.clone()));
        } else {
            self.cs.cs.cs[1].constrain(&gate_io, T::from(polyop.clone()));
        }
        
        self.ops[round].push(Operation::Poly(polyop.allocate(i, output.clone())));

        output
    }

    pub fn apply_pub(&mut self, round : usize, polyop: PolyOp<'a, F>, i: Vec<Variable>) -> Vec<Variable> {
        assert!(!self.finalized, "Circuit is already built.");
        assert!(round < self.ops.len(), "The round is too large.",);
        assert!(i.len() == polyop.i, "Incorrect amount of inputs at operation {}", self.ops.len());
        for v in &i {
            assert!(v.round() <= round, "Argument of an operation {} ; {} is in round {}", round, self.ops[round].len(), v.round())
        }
        let mut output = vec![];
        for _ in 0..polyop.o {
            output.push( self.cs.alloc_pub_internal(round) );
        }

        let mut gate_io = vec![Variable::Public(0,0)];
        gate_io.append(&mut i.clone());
        gate_io.append(&mut output.clone());
        if polyop.d == 0 {panic!("Operation {} has degree 0, which is banned.", self.ops.len())}
        if polyop.d > self.max_degree {panic!("Degree of operation {} is too large!", self.ops.len())};
        if polyop.d == 1 {
            self.cs.cs.cs[0].constrain(&gate_io, T::from(polyop.clone()));
        } else {
            self.cs.cs.cs[1].constrain(&gate_io, T::from(polyop.clone()));
        }
        
        self.ops[round].push(Operation::Poly(polyop.allocate(i, output.clone())));

        output
    }

    pub fn constrain(&mut self, inputs: &[Variable], gate: T) -> (){
        if gate.d() == 0 {panic!("Trying to constrain with gate of degree 0.")};
        let mut tmp = 1;
        if gate.d() == 1 {tmp = 0}
        self.cs.cs.cs[tmp].constrain(inputs, gate);
    }


    pub fn finalize(&mut self) -> () {
        assert!(!self.finalized, "Circuit is already built.");
        self.finalized = true;
    }

    /// Executes the circuit up from the current program counter to round k.
    pub fn execute(&mut self, round: usize) -> () {
        assert!(self.finalized, "Must finalize circuit before executing it.");
        if self.round_counter > round {
            panic!("Execution is at round finished round {}, attempt to execute up to round {}", self.round_counter, round)
            }
        while (self.round_counter <= round) {
            for op in &self.ops[self.round_counter]{
                match op {
                    Operation::Poly(polyop) => {
                        let input : Vec<_> = polyop.i.iter().map(|x|self.cs.getvar(*x)).collect();
                        let output = (&polyop.op.f)(F::ONE, &input);
                        polyop.o.iter().zip(output.iter()).map(|(i,v)| self.cs.setvar(*i, *v)).count();
                    }
                    Operation::Adv(adv) => {
                        let input : Vec<_> = adv.ivar.iter().map(|x|self.cs.getvar(*x)).collect();
                        let input_ext : Vec<_> = adv.iext.iter().map(|x|*x.get().unwrap()).collect();
                        let output = (&adv.adv.f)(&input, &input_ext);
                        adv.o.iter().zip(output.iter()).map(|(i,v)| {
                            self.cs.setvar(*i, *v)
                        }).count();
                    }
                }
            }
            self.round_counter += 1;
        }
    }
}