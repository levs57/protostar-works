use std::{cmp::max, marker::PhantomData};

use ff::PrimeField;

use crate::gate::Gate;

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

impl Variable {
    pub fn round(&self) -> usize {
        match self {
            Self::Private(r,_) => *r,
            Self::Public(r, _) => *r,
        }
    }
}

#[derive(Clone)]
pub struct Constraint<F: PrimeField, T : Gate<F>>{
    pub inputs: Vec<Variable>,
    pub gate: T,
    _marker: PhantomData<F>,
}

#[derive(Clone)]
pub struct ConstraintGroup<F: PrimeField, T : Gate<F>>{
    pub entries: Vec<Constraint<F, T>>,
    pub kind: CommitKind,
    pub num_rhs: usize,
    pub degree: usize,
}

impl<F: PrimeField, T : Gate<F>> ConstraintGroup<F, T>{
    pub fn constrain(&mut self, inputs: &[Variable], gate: T) {
        assert!(gate.d() <= self.degree, "Constraint degree is too large for this group.");
        assert!(gate.i() == inputs.len(), "Invalid amount of arguments supplied.");
        self.num_rhs += gate.o();
        self.entries.push(Constraint{inputs : inputs.to_vec(), gate, _marker : PhantomData});
    }
}

#[derive(Clone)]
pub struct VarGroup{
    pub privs: usize,
    pub pubs: usize,
}

impl VarGroup {
    pub fn new() -> Self{
        VarGroup{privs: 0, pubs: 0}
    }
}

#[derive(Clone)]
pub struct ConstraintSystem<F: PrimeField, T : Gate<F>>{
    pub vars: Vec<VarGroup>,
    pub cs : Vec<ConstraintGroup<F, T>>,
}

impl<F: PrimeField, T : Gate<F>> ConstraintSystem<F, T>{
    /// Returns an emptry constraint system.
    pub fn new(num_rounds:usize) -> Self {
        Self{vars: vec![VarGroup::new(); num_rounds], cs: vec![]}
    }

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

    /// Returns index of a constraint group. Can not return the reference on a group itself because crab god angry.
    pub fn add_constr_group(&mut self, kind: CommitKind, degree: usize) -> usize {
        match kind {
            CommitKind::Zero => assert!(degree == 1, "Zero commit kind is only usable for linear constraints."),
            _ => assert!(degree > 1, "Nonzero commit kinds are only for degree > 1"),
        }
        self.cs.push(ConstraintGroup::<F, T>{entries: vec![], kind, num_rhs: 0, degree});
        self.cs.len()-1
    }

    /// Computes global id of a variable.
    pub fn var_global_id(&self, v: Variable, partial_sums: &Vec<usize>) -> usize{
        match v {
            Variable::Public(a, b) => partial_sums[a]+b,
            Variable::Private(a, b) => partial_sums[a]+self.vars[a].pubs+b,
        }
    }

    /// Computes partial sums of variable counts.
    pub fn varcount_partial_sums(&self) -> Vec<usize> {
        let mut ret = vec![];
        let mut tmp = 0;
        for i in 0..self.vars.len() {
            tmp = tmp + self.vars[i].privs + self.vars[i].pubs;
            ret.push(tmp);
        }
        ret
    }

    /// Returns size of the error vector.
    pub fn num_rhs(&self) -> usize {
        let mut num_rhs = 0;
        for cg in &self.cs {
            match cg.kind {
                CommitKind::Zero => (),
                CommitKind::Trivial => panic!("Unexpected trivial commitment kind."),
                CommitKind::Group => {num_rhs += cg.num_rhs},
            }
        }
        num_rhs
    }

    pub fn max_deg(&self) -> usize{
        let mut max_deg = 0;
        for cg in &self.cs {
            max_deg = max(max_deg, cg.degree);
        }
        max_deg
    }

    // fn div_ceil(a: usize, b: usize) -> usize{
    //     (a+b-1)/b
    // }

    // Returns a protostar transform of the constraint system.
    // Round combining is not implemented, but you can add additional constraints after doing protostar transform.
    // pub fn protostarize<'a>(&'a self) -> Self where F:FieldUtils, T:Gatebb<'a, F>{

    //     assert!(self.vars[0].pubs>0, "Constraint system must have 1st input.");
    //     let mut protostar = ConstraintSystem::<F, T>{vars : self.vars.clone(), cs : vec![]};

    //     let max_deg = self.max_deg();
    //     let num_rhs = self.num_rhs();


    //     protostar.new_round(); // Creates a new round in which we will allocate our protostar stuff.

    //     let sq = (num_rhs+1).sqrt(); // Ceil of a square root.

    //     let one = Variable::Public(0, 0);
    //     let mut alphas = vec![one];
    //     let mut betas = vec![one];

    //     let lin = protostar.add_constr_group(CommitKind::Zero, 1);
    //     let quad = protostar.add_constr_group(CommitKind::Group, 2);
    //     let triv = protostar.add_constr_group(CommitKind::Trivial, max_deg+2);

    //     for i in 1..sq {
    //         let tmp = {
    //             match i {
    //                 1 => protostar.alloc_pub(),
    //                 _ => protostar.alloc_priv(),
    //                 }
    //             };
    //         alphas.push(tmp);
    //     }

    //     for i in 1..sq{
    //         betas.push(protostar.alloc_priv());
    //     }


    //     let quad = &mut protostar.cs[quad];

    //     for i in 2..sq {
    //         quad.constrain(
    //             &[alphas[1], alphas[i-1], alphas[i], one],
    //             Box::new(Gatebb::new(
    //                 2,
    //                 4,
    //                 1,
    //                 Box::new(|v|{
    //                     let alpha = v[0];
    //                     let alpha_prev = v[1];
    //                     let alpha_curr = v[2];
    //                     let one = v[3];

    //                     vec![alpha * alpha_prev - alpha_curr * one]

    //                 })
    //             )) 
    //         );
    //         quad.constrain(
    //             &[betas[1], betas[i-1], betas[i], one],
    //             Box::new(Gatebb::new(
    //                 2,
    //                 4,
    //                 1,
    //                 Box::new(|v|{
    //                     let beta = v[0];
    //                     let beta_prev = v[1];
    //                     let beta_curr = v[2];
    //                     let one = v[3];

    //                     vec![beta * beta_prev - beta_curr * one]

    //                 })
    //             )) 
    //         );
    //     }
    //     quad.constrain(
    //         &[alphas[1], alphas[sq-1], betas[1], one],
    //         Box::new(Gatebb::new(
    //             2,
    //             4,
    //             1,
    //             Box::new(|v|{
    //                 let alpha = v[0];
    //                 let alpha_last = v[1];
    //                 let beta = v[2];
    //                 let one = v[3];

    //                 vec![alpha * alpha_last - beta * one]
    //             })
    //         )

    //         )
    //     );

    //     let f = Box::new(|v : &[F]|{            
    //         let partial_sums = self.varcount_partial_sums(); // Aux data for global variable id.
    //         let num_lhs = partial_sums[partial_sums.len()-1];
    //         let num_rhs = self.num_rhs();
    //         let (wtns_normal, wtns_greek_letters) = v.split_at(num_lhs);
    //         let sq = (num_rhs+1).sqrt();
    //         let (tmp1, tmp2) = wtns_greek_letters.split_at(sq-1);
    //         let mut alphas = vec![wtns_normal[0]];
    //         let mut betas = vec![wtns_normal[0]];
    //         alphas.append(&mut ((*tmp1).to_vec()));
    //         betas.append(&mut ((*tmp2).to_vec()));

    //         let mut rhs = vec![];
    //         for cg in &self.cs {
    //             match cg.kind {
    //                 CommitKind::Zero => continue,
    //                 CommitKind::Trivial => panic!("Unexpected group with trivial commitment kind. Pre-protostar trivial commitment kinds are currently unsupported."),
    //                 _ => (),
    //             }                
                
    //             for constr in &cg.entries {
    //                 let args: Vec<_> = constr.inputs.iter()
    //                         .map(|var|v[self.var_global_id(*var, &partial_sums)])
    //                         .collect();
    //                 rhs.append(&mut constr.gate.exec(&args));

    //             }
    //         }
    //         let mut acc = F::ZERO;
    //         for i in 0..rhs.len(){
    //             acc += rhs[i]*alphas[i%sq]*betas[i/sq]
    //         }
    //         vec![acc]
    //     });

    //     let mut args = vec![];
    //     for (round, vg) in protostar.vars.iter().enumerate(){ // The variables are put into the gate round-by-round, public first.
    //         args.append(&mut
    //             (0..vg.pubs).map(|id|Variable::Public(round, id)).chain((0..vg.privs).map(|id|Variable::Private(round,id))).collect()
    //         )
    //     }

    //     let triv = &mut protostar.cs[triv];

    //     triv.constrain(
    //         &args, // We need to constrain everything.
    //         Box::new(Gatebb::<'a>::new_unchecked(
    //             max_deg,
    //             args.len(),
    //             1,
    //             f,
    //         ))
    //     );

    //     let lin = &mut protostar.cs[lin];

    //     // This is ugly, but I don't want to bother with DynClone for now.
    //     for cg in &self.cs {
    //         match cg.kind {
    //             CommitKind::Trivial => {
    //                 for constr in &cg.entries {
    //                     lin.constrain(&constr.inputs, Box::new(Gatebb::new_unchecked(
    //                         constr.gate.d(),
    //                         constr.gate.i(),
    //                         constr.gate.o(),
    //                         Box::new(|x|constr.gate.exec(x))))
    //                     )
    //                 } 
    //             },
    //             _ => ()
    //         }
    //     }

    //     protostar

    // }
}
