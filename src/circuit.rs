use std::{iter::repeat, cmp::max};

use ff::PrimeField;
use num_traits::{Pow, pow};
use rand_core::OsRng;

use crate::{witness::CSWtns, gate::{Gate, Gatebb, RootsOfUnity, AdjustedGate}, constraint_system::{Variable, ConstraintSystem, CommitKind, VarGroup}};

pub enum ExecMode{
    Constrain,
    Execute,
}

/// A polynomial operation. First argument is a relaxation factor, passed separately.
/// Must be homogeneous (current API limitation, need to remove it at some point).
/// In a gate, it is by convention transformed to be first input.
pub struct PolyOp<'a, F:PrimeField>{
    pub d: usize,
    pub i: usize,
    pub o: usize,
    pub f: Box<dyn Fn(F, &[F]) -> Vec<F> + 'a>,
}

impl<'a, F:PrimeField> PolyOp<'a, F> {
    pub fn new(d: usize, i: usize, o: usize, f: Box<dyn Fn(F, &[F]) -> Vec<F> + 'a>) -> Self{
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
 
        PolyOp { d, i, o, f }
    }

    pub fn into_gate(&'a self) -> Gatebb<'a, F> {
        let d = self.d;
        let i = self.i + 1 + self.o;
        let o = self.o;

        let f = |args: &[F]| {
            let (inputs, outputs) = args.split_at(self.i);
            let (one, inputs) = inputs.split_at(1);
            let one = one[0];
            let results = (self.f)(one, &inputs);
            let onepow = one.pow([(self.d-1) as u64]);
            results.iter().zip(outputs.iter()).map(|(inp, out)|*inp-*out*onepow).collect()
        };

        Gatebb::new(d, i, o, Box::new(f))
    }
}

pub trait AdviceCtx<'a, F: PrimeField>{
}

pub struct Circuit<'a, F: PrimeField, Ctx: AdviceCtx<'a, F>> {
    pub cs: CSWtns<'a, F>,
    pub mode: ExecMode,

    pub max_degree: usize,

    pub current_exec_round: usize,
    pub vars_curr: VarGroup,

    pub ctx: Ctx,
}

impl<'a, F:PrimeField+RootsOfUnity, Ctx:AdviceCtx<'a, F>> Circuit<'a, F, Ctx>{
    pub fn new(max_degree:usize, ctx: Ctx) -> Self {
        let mut cs = ConstraintSystem::new();
        cs.add_constr_group(CommitKind::Group, max_degree);
        Circuit{cs : CSWtns::new(cs),
                mode : ExecMode::Constrain,
                max_degree,
                vars_curr : VarGroup{privs: 0, pubs: 1},
                current_exec_round: 0,
                ctx
            }
    }
    
    fn alloc_pub(&mut self) -> Variable {
        match self.mode {
            ExecMode::Constrain => self.cs.cs.alloc_pub(),
            ExecMode::Execute => {self.vars_curr.pubs += 1; Variable::Public(self.current_exec_round, self.vars_curr.pubs-1)},
        }
    }

    fn alloc_priv(&mut self) -> Variable {
        match self.mode {
            ExecMode::Constrain => self.cs.cs.alloc_priv(),
            ExecMode::Execute => {self.vars_curr.privs += 1; Variable::Private(self.current_exec_round, self.vars_curr.privs-1)},
        }    
    }

    pub fn constrain<T: 'a + Gate<'a, F> + Sized>(&mut self, inputs: &[Variable], gate: T) -> (){
        match self.mode {
            ExecMode::Constrain => self.cs.cs.cs[0].constrain(inputs, Box::new(gate.adjust(self.max_degree))),
            ExecMode::Execute => (),
        }
    }

    pub fn new_round(&mut self) -> (){
        match self.mode {
            ExecMode::Constrain => self.cs.cs.new_round(),
            ExecMode::Execute => {
                self.current_exec_round += 1;
                self.vars_curr = VarGroup{privs:0, pubs:0};
            },
        }
    }

    pub fn apply(&mut self, op: &'a PolyOp<'a, F>, args: &[Variable]) -> Vec<Variable>{
        match self.mode {
            ExecMode::Constrain => {
                assert!(args.len() == op.i+1);
                let mut inputs = args.to_vec();
                let o = op.o;
                let mut outputs = vec![];
                for _ in 0..o {
                    outputs.push(self.alloc_priv());
                }
                inputs.append(&mut outputs.clone());
                self.constrain(&inputs, op.into_gate());
                outputs
            },
            ExecMode::Execute => {
                let fetch_args : Vec<_> = args.iter().map(|i|self.cs.getvar(*i)).collect();
                let outputs = (op.f)(F::ONE, &fetch_args);
                let mut ret = vec![];
                for val in outputs {
                    let tmp = self.alloc_priv();
                    self.cs.setvar(tmp, val);
                    ret.push(tmp);
                }
                ret
            }
        }
    }

    pub fn advice_priv(&mut self, f: Box<dyn 'a + Fn(&Self, Variable) -> F>) -> Variable{
        let var = self.alloc_priv();
        match self.mode {
            ExecMode::Constrain => {
                ()
            }
            ExecMode::Execute => {
                self.cs.setvar(var, f(self, var))
            }
        };
        var
    }

    pub fn advice_pub(&mut self, f: Box<dyn 'a + Fn(&Self, Variable) -> F>) -> Variable{
        let var = self.alloc_pub();
        match self.mode {
            ExecMode::Constrain => {
                ()
            }
            ExecMode::Execute => {
                self.cs.setvar(var, f(self, var))
            }
        };
        var
    }
}

