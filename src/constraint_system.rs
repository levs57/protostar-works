use std::cmp::max;

use ff::PrimeField;
use num_integer::Roots;

use crate::gate::{Gate, Gatebb, RootsOfUnity};

#[derive(Clone, Copy)]
pub enum VarKind {
    Public,
    Challenge(usize), // usable after round
    Round(usize),
}

pub enum CommitKind {
    Trivial,
    Group,
}

#[derive(Clone, Copy)]
pub struct Variable{
    n: usize,
    kind: VarKind,
}

pub struct Constraint<'a, F: PrimeField>{
    inputs: Vec<usize>,
    gate: Box<dyn 'a + Gate<'a, F>>,
    kind: CommitKind,
}

pub struct ConstraintSystem<'a, F: PrimeField>{
    wtns: Vec<Variable>,
    constraints : Vec<Constraint<'a, F>>,
    max_degree : usize,
    num_rounds : usize,
    num_rhs : usize,
}

impl<'a, F: PrimeField + RootsOfUnity> ConstraintSystem<'a, F>{
    /// Returns a constraint system with a single public variable corresponding to 1.
    /// You would need different API for supernovaish constructions. Too lazy to do it now.
    pub fn new() -> Self {
        let mut tmp = Self{wtns : vec![], constraints : vec![], max_degree : 0, num_rounds : 1, num_rhs : 0};
        let _one = tmp.alloc(VarKind::Public);
        tmp
    }

    /// Allocates a new variable.
    pub fn alloc(&mut self, kind: VarKind) -> &Variable{
        match kind {
            VarKind::Round(k) => self.num_rounds = max(self.num_rounds, k+1),
            VarKind::Challenge(k) => self.num_rounds = max(self.num_rounds, k+2),
            _ => (),
        }
        let n = self.wtns.len();
        self.wtns.push(Variable{n, kind});
        return &self.wtns[n]
    }

    /// Constrains a collection of variables. These variables must be from this constraint system -- UNSAFE API!
    /// Always uses group-like commitment for the error term.
    pub fn constrain(&mut self, args: &[Variable], gate: Box<dyn 'a + Gate<'a, F>>){
        let d = gate.d();
        self.max_degree = max(self.max_degree, d);
        self.num_rhs += gate.o();
        self.constraints.push(Constraint{ inputs : args.iter().map(|x|x.n).collect(), gate, kind : CommitKind::Group });
    }

    /// Adds a constraint with trivial commit kind.
    pub fn constrain_expose(&mut self, args: &[Variable], gate: Box<dyn 'a + Gate<'a, F>>){
        let d = gate.d();
        self.max_degree = max(self.max_degree, d);
        self.num_rhs += gate.o();
        self.constraints.push(Constraint{ inputs : args.iter().map(|x|x.n).collect(), gate, kind : CommitKind::Trivial });
    }

    /// Returns a single gate computing entire rhs on the witness vector. Uniformizes the constraint system.
    /// Will not work if there are already constraints with public commit type.
    /// Warning: separate reduction of linear constraints is NOT IMPLEMENTED. Do not use linear constraints.
    pub fn as_gate(&'a self) -> Gatebb<'a, F> {
        let f = |inputs: &[F]| {
            let mut ones = vec![inputs[0]]; // powers of the relaxation factor
            for _ in 1..self.max_degree {
                ones.push(ones[0] * ones[ones.len()-1])
            }
            let mut ret = vec![];
            for constr in &self.constraints {
                assert!(match constr.kind {CommitKind::Group => true, _ => false}, "Can not uniformize constraints with trivial commitment scheme.");
                let deg_offset = self.max_degree-constr.gate.d();
                ret.append(
                    {
                        let tmp : Vec<_> = self.wtns.iter().map(|x|inputs[x.n]).collect();
                        &mut constr.gate.exec(
                            &tmp
                        ).iter()
                        .map(|x|{
                            match deg_offset {
                                0 => *x,
                                offset => *x*ones[offset-1],
                            }
                        })
                        .collect()
                    }
                );                  
            }
            ret
        };
        Gatebb::<'a>::new_unchecked(
            self.max_degree,
            self.wtns.len(),
            self.constraints.iter().fold(0, |acc, v|acc+v.gate.o()),
            Box::new(f)
        )
    }

    fn div_ceil(a: usize, b: usize) -> usize{
        (a+b-1)/b
    }

    /// Returns a protostar transform of the constraint system.
    /// Round combining is not implemented, but you can add additional constraints after doing protostar transform.
    pub fn protostarize(&'a self) -> Self {

        let mut protostar = ConstraintSystem::<'a, F>{wtns : self.wtns.clone(), constraints : vec![], max_degree: self.max_degree+2, num_rounds: self.num_rounds+1, num_rhs: 0};

        let num_constraints = self.constraints.len();
        let sq = (num_constraints+1).sqrt(); // ceil of a square root

        let one = protostar.wtns[0].clone();
        let mut alphas = vec![one];
        let mut betas = vec![one];

        for i in 1..sq {
            {
                let tmp = protostar.alloc(VarKind::Challenge(self.num_rounds-1)).clone();
                alphas.push(tmp);
            }
            {
                let tmp = protostar.alloc(VarKind::Challenge(self.num_rounds-1)).clone();
                betas.push(tmp);
            }
        }

        for i in 2..sq {
            protostar.constrain(
                &[alphas[1], alphas[i-1], alphas[i], one],
                Box::new(Gatebb::new(
                    2,
                    4,
                    1,
                    Box::new(|v|{
                        let alpha = v[0];
                        let alpha_prev = v[1];
                        let alpha_curr = v[2];
                        let one = v[3];

                        vec![alpha * alpha_prev - alpha_curr * one]

                    })
                )) 
            );
            protostar.constrain(
                &[betas[1], betas[i-1], betas[i], one],
                Box::new(Gatebb::new(
                    2,
                    4,
                    1,
                    Box::new(|v|{
                        let beta = v[0];
                        let beta_prev = v[1];
                        let beta_curr = v[2];
                        let one = v[3];

                        vec![beta * beta_prev - beta_curr * one]

                    })
                )) 
            );
        }

        let f = Box::new(|v : &[F]|{
            let (wtns_normal, wtns_greek_letters) = v.split_at(self.wtns.len());
            let sq = (self.num_rhs+1).sqrt();
            let (alphas, tmp) = wtns_greek_letters.split_at(sq);
            let mut betas = vec![wtns_normal[0]];
            betas.append(&mut ((*tmp).to_vec()));
            let rhs = self.as_gate().exec(wtns_normal);
            let mut acc = F::ZERO;
            for i in 0..rhs.len(){
                acc += rhs[i]*alphas[i%sq]*betas[i/sq]
            }
            vec![acc]
        });

        let args = protostar.wtns.clone();
        protostar.constrain_expose(
            &args, // We need to constrain everything.
            Box::new(Gatebb::<'a>::new_unchecked(
                protostar.max_degree,
                protostar.wtns.len(),
                1,
                f,
            ))
        );

        protostar

    }
}
