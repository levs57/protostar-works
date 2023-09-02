use std::{cmp::max, rc::Weak};

use ff::PrimeField;
use num_integer::Roots;

use crate::gate::{Gate, Gatebb, RootsOfUnity};

#[derive(Clone, Copy)]

pub enum CommitKind {
    Trivial,
    Group,
    Zero, // Used in cases where we do not need to commit.
}

#[derive(Clone, Copy)]
pub enum Variable{
    Private(usize, usize), // Private variables.
    Public(usize, usize), // Public variables, including challenges.
}
pub struct Constraint<'a, F: PrimeField>{
    inputs: Vec<Variable>,
    gate: Box<dyn 'a + Gate<'a, F>>,
}

pub struct ConstraintGroup<'a, F: PrimeField>{
    entries: Vec<Constraint<'a, F>>,
    kind: CommitKind,
    num_rhs: usize,
    degree: usize,
}

impl<'a, F: PrimeField> ConstraintGroup<'a, F>{
    pub fn constrain(&mut self, inputs: &[Variable], gate: Box<dyn 'a + Gate<'a, F>>) {
        assert!(gate.d() <= self.degree, "Constraint degree is too large for this group.");
        assert!(gate.i() == inputs.len(), "Invalid amount of arguments supplied.");
        self.num_rhs += gate.o();
        self.entries.push(Constraint{inputs : inputs.to_vec(), gate});
    }
}

#[derive(Clone)]
pub struct VarGroup{
    privs: usize,
    pubs: usize,
}

impl VarGroup {
    pub fn new() -> Self{
        VarGroup{privs: 0, pubs: 0}
    }
}

pub struct ConstraintSystem<'a, F: PrimeField>{
    vars: Vec<VarGroup>,
    cs : Vec<ConstraintGroup<'a, F>>,
}

impl<'a, F: PrimeField + RootsOfUnity> ConstraintSystem<'a, F>{
    /// Returns a constraint system with a single public variable corresponding to 1.
    /// You would need different API for supernovaish constructions. Too lazy to do it now.
    // pub fn new() -> Self {
    //     let mut tmp = Self{wtns : vec![], constraints : vec![], max_degree : 0, num_rounds : 1, num_rhs : 0};
    //     let _one = tmp.alloc(VarKind::Public);
    //     tmp
    // }

    pub fn num_rounds(&self) -> usize{
        self.vars.len()
    }

    pub fn new_round(&mut self) -> () {
        self.vars.push(VarGroup::new());
    }

    pub fn alloc_pub_internal(&mut self, round: usize) -> Variable{
        let ret = self.vars[round].pubs;
        self.vars[round].pubs+=1;
        Variable::Public(round, ret)
    }

    pub fn alloc_priv_internal(&mut self, round: usize) -> Variable{
        let ret = self.vars[round].privs;
        self.vars[round].privs+=1;
        Variable::Private(round, ret)
    }

    pub fn alloc_pub(&mut self) -> Variable{
        self.alloc_pub_internal(self.num_rounds()-1)
    }

    pub fn alloc_priv(&mut self) -> Variable{
        self.alloc_priv_internal(self.num_rounds()-1)
    }

    pub fn add_constr_group(&mut self, kind: CommitKind, degree: usize) -> ConstraintGroup<'a, F>{
        match kind {
            CommitKind::Zero => assert!(degree == 1, "Zero commit kind is only usable for linear constraints."),
            _ => assert!(degree > 1, "Nonzero commit kinds are only for degree > 1"),
        }
        self.cs.push(ConstraintGroup::<'a,F>{entries: vec![], kind, num_rhs: 0, degree});
        let l = self.cs.len();
        self.cs[l-1]
    }


    /// Returns a single gate computing entire rhs on the witness vector. Uniformizes the constraint system.
    /// Will not work if there are already constraints with public commit type.
    /// Warning: separate reduction of linear constraints is NOT IMPLEMENTED. Do not use linear constraints.
    // pub fn as_gate(&'a self) -> Gatebb<'a, F> {
    //     let mut max_degree = 0;
    //     for csgroup in self.cs {
    //         max_degree = max(max_degree, csgroup.degree)
    //     }
    //     let f = |inputs: &[F]| {
    //         let mut ones = vec![inputs[0]]; // powers of the relaxation factor
    //         for _ in 1..max_degree {
    //             ones.push(ones[0] * ones[ones.len()-1])
    //         }
    //         let mut ret = vec![];
    //         for csgroup in self.cs{
    //             for constr in csgroup.entries {
    //                 assert!(match constr.kind {CommitKind::Group => true, _ => false}, "Can not uniformize constraints with trivial commitment scheme.");
    //                 let deg_offset = self.max_degree-constr.gate.d();
    //                 ret.append(
    //                     {
    //                         let tmp : Vec<_> = self.wtns.iter().map(|x|inputs[x.n]).collect();
    //                         &mut constr.gate.exec(
    //                             &tmp
    //                         ).iter()
    //                         .map(|x|{
    //                             match deg_offset {
    //                                 0 => *x,
    //                                 offset => *x*ones[offset-1],
    //                             }
    //                         })
    //                         .collect()
    //                     }
    //                 );                  
    //             }
    //         }
    //         ret
    //     };
    //     Gatebb::<'a>::new_unchecked(
    //         self.max_degree,
    //         self.wtns.len(),
    //         self.num_rhs,
    //         Box::new(f)
    //     )
    // }

    fn div_ceil(a: usize, b: usize) -> usize{
        (a+b-1)/b
    }

    /// Returns a protostar transform of the constraint system.
    /// Round combining is not implemented, but you can add additional constraints after doing protostar transform.
    pub fn protostarize(&'a self) -> Self {

        assert!(self.vars[0].pubs>0, "Constraint system must have 1st input.");
        let mut protostar = ConstraintSystem::<'a, F>{vars : self.vars.clone(), cs : vec![]};

        let mut max_deg = 0;
        let mut num_rhs = 0;

        for cg in self.cs {
            max_deg = max(max_deg, cg.degree);
            num_rhs += cg.num_rhs;
        }

        let mut num_lhs = 0;

        for vg in self.vars{
            num_lhs += (vg.privs+vg.pubs);
        }

        protostar.new_round(); // Creates a new round in which we will allocate our protostar stuff.

        let sq = (num_rhs+1).sqrt(); // Ceil of a square root.

        let one = Variable::Public(0, 0);
        let mut alphas = vec![one];
        let mut betas = vec![one];

        let lin = protostar.add_constr_group(CommitKind::Zero, 1);
        let quad = protostar.add_constr_group(CommitKind::Group, 2);
        let triv = protostar.add_constr_group(CommitKind::Trivial, max_deg+2);

        for i in 1..sq {
            let tmp = {
                match i {
                    1 => protostar.alloc_pub(),
                    _ => protostar.alloc_priv(),
                    }
                };
            alphas.push(tmp);
        }

        for i in 1..sq{
            betas.push(protostar.alloc_priv());
        }

        for i in 2..sq {
            quad.constrain(
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
            quad.constrain(
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
        quad.constrain(
            &[alphas[1], alphas[sq-1], betas[1], one],
            Box::new(Gatebb::new(
                2,
                4,
                1,
                Box::new(|v|{
                    let alpha = v[0];
                    let alpha_last = v[1];
                    let beta = v[2];
                    let one = v[3];

                    vec![alpha * alpha_last - beta * one]
                })
            )

            )
        );


        // assumptions:
        // ONE lives in 0-th input
        // total argsize num_lhs + 2sq-2
        let f = Box::new(|v : &[F]|{
            let (wtns_normal, wtns_greek_letters) = v.split_at(num_lhs);
            let sq = (num_rhs+1).sqrt();
            let (tmp1, tmp2) = wtns_greek_letters.split_at(sq-1);
            let mut alphas = vec![wtns_normal[0]];
            let mut betas = vec![wtns_normal[0]];
            alphas.append(&mut ((*tmp1).to_vec()));
            betas.append(&mut ((*tmp2).to_vec()));
            let mut rhs = vec![];
            for cg in self.cs {
                match cg.kind {
                    CommitKind::Zero => continue,
                    CommitKind::Trivial => panic!("Unexpected group with trivial commitment kind. Pre-protostar trivial commitment kinds are currently unsupported."),
                    _ => (),
                }

                let one_pow = wtns_normal[0].pow([(max_deg - cg.degree)]);
                
                
                for constr in cg.entries{}
            }
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
