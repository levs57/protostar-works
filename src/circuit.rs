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

        Gatebb::new(d, i, o, Box::new(f))    
    }
}

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
 pub struct AdviceAllocated<'a, F: PrimeField> {
    adv: Advice<'a, F>,
    ivar: Vec<Variable>,
    iext: Vec<&'a ExternalValue<F>>,
    o: Vec<Variable>,
 }

pub enum Operation<'a, F: PrimeField> {
    Poly(PolyOpAllocated<'a, F>),
    Adv(AdviceAllocated<'a, F>),
    RoundLabel(usize),
}

pub struct Circuit<'a, F: PrimeField, T:Gate<F> + From<PolyOp<'a, F>>> {
    pub cs: CSWtns<F, T>,
    pub ops: Vec<Operation<'a, F>>,
    pub max_degree: usize,
    pub finalized: bool,
    pub pc : usize,
}

impl<'a, F: PrimeField + RootsOfUnity, T: Gate<F> + From<PolyOp<'a, F>>> Circuit<'a, F, T>{
    pub fn new(max_degree: usize) -> Self {
        let mut cs = ConstraintSystem::new();
        cs.add_constr_group(CommitKind::Zero, 1);
        cs.add_constr_group(CommitKind::Group, max_degree);
        let mut prep = Self{
                cs : CSWtns::new(cs),
                ops: vec![],
                max_degree,
                finalized: false,
                pc : 0,
            };
        let adv = Advice::new(0,0,1, Rc::new(|_,_|vec![F::ONE]));
        let tmp = prep.advice_pub(adv, vec![], vec![]);
        match tmp[0] {
            Variable::Public(0,0) => (),
            _ => panic!("One has allocated incorrectly. This should never fail. Abort mission."),
        };
        prep
    }

    pub fn advice(&mut self, adv: Advice<'a, F>, ivar: Vec<Variable>, iext: Vec<&'a ExternalValue<F>>) -> Vec<Variable> {
        assert!(!self.finalized, "Circuit is already built.");
        assert!(ivar.len() == adv.ivar, "Incorrect amount of inputs at operation {}", self.ops.len());
        assert!(iext.len() == adv.iext, "Incorrect amount of advice inputs at operation {}", self.ops.len());
        let mut output = vec![];
        for _ in 0..adv.o {
            output.push( self.cs.alloc_priv() );
        }
        self.ops.push(Operation::Adv(adv.allocate(ivar, iext, output.clone())));
        output
    }

    pub fn advice_pub(&mut self, adv: Advice<'a, F>, ivar: Vec<Variable>, iext: Vec<&'a ExternalValue<F>>) -> Vec<Variable> {
        assert!(!self.finalized, "Circuit is already built.");
        assert!(ivar.len() == adv.ivar, "Incorrect amount of inputs at operation {}", self.ops.len());
        assert!(iext.len() == adv.iext, "Incorrect amount of advice inputs at operation {}", self.ops.len());
        let mut output = vec![];
        for _ in 0..adv.o {
            output.push( self.cs.alloc_pub() );
        }
        self.ops.push(Operation::Adv(adv.allocate(ivar, iext, output.clone())));
        output
    }

    pub fn apply(&mut self, polyop: PolyOp<'a, F>, i: Vec<Variable>) -> Vec<Variable> {
        assert!(!self.finalized, "Circuit is already built.");
        assert!(i.len() == polyop.i, "Incorrect amount of inputs at operation {}", self.ops.len());
        let mut output = vec![];
        for _ in 0..polyop.o {
            output.push( self.cs.alloc_priv() );
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
        
        self.ops.push(Operation::Poly(polyop.allocate(i, output.clone())));

        output
    }

    pub fn apply_pub(&mut self, polyop: PolyOp<'a, F>, i: Vec<Variable>) -> Vec<Variable> {
        assert!(!self.finalized, "Circuit is already built.");
        assert!(i.len() == polyop.i, "Incorrect amount of inputs at operation {}", self.ops.len());
        let mut output = vec![];
        for _ in 0..polyop.o {
            output.push( self.cs.alloc_pub() );
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
        
        self.ops.push(Operation::Poly(polyop.allocate(i, output.clone())));

        output
    }

    pub fn constrain(&mut self, inputs: &[Variable], gate: T) -> (){
        if gate.d() == 0 {panic!("Trying to constrain with gate of degree 0.")};
        let mut tmp = 1;
        if gate.d() == 1 {tmp = 0}
        self.cs.cs.cs[tmp].constrain(inputs, gate);
    }


    pub fn next_round(&mut self) -> () {
        assert!(!self.finalized, "Circuit is already built.");
        self.ops.push(Operation::RoundLabel(self.cs.cs.num_rounds()-1));
        self.cs.cs.new_round();
    }

    pub fn finalize(&mut self) -> () {
        assert!(!self.finalized, "Circuit is already built.");
        self.ops.push(Operation::RoundLabel(self.cs.cs.num_rounds()-1));
        self.finalized = true;
    }

    /// Executes the circuit up from the current program counter to round k.
    pub fn execute(&mut self, round: usize) -> () {
        assert!(self.finalized, "Must finalize circuit before executing it.");
        if self.pc>0 {
            match self.ops[self.pc-1] {
                Operation::RoundLabel(r) => assert!(r < round, "Execution has already finished round {}, attempt to execute up to round {}", r, round),
                _ => panic!("Program counter in a wrong place. This should never happen."),
            }
        }
        loop {
            match &self.ops[self.pc] {
                Operation::RoundLabel(x) => if *x==round {break},
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
            self.pc += 1;
        }
    }
}