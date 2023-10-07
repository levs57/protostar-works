#[cfg(test)]
mod tests {
    use std::{rc::Rc, iter::repeat};

    use crate::{gate::Gatebb, constraint_system::{Variable, Visibility}, circuit::{Circuit, ExternalValue, PolyOp, Advice, Build}, gadgets::{poseidon::{poseidon_gadget, Poseidon, poseidon_gadget_mixstrat}, bits::bit_decomposition_gadget, ecmul::{add_proj, double_proj, EcAffinePoint, escalarmul_gadget_9}, range::{rangecheck, limb_decompose_gadget, lagrange_choice, lagrange_choice_batched, VarSmall, choice_gadget}, nonzero_check::nonzero_gadget}};
    use ff::{PrimeField, Field};
    use group::{Group, Curve};
    use halo2curves::{bn256, grumpkin, CurveAffine, CurveExt};
    use rand_core::OsRng;
    use crate::utils::poly_utils::{check_poly, find_degree};
    use crate::utils::field_precomp::FieldUtils;
    
    type F = bn256::Fr;
    type C = grumpkin::G1;

    type Fq = <C as CurveExt>::ScalarExt;
    
    // #[test]
    
    // fn test_cross_terms() {
    
    //     for d in 2..10{
    //         let f = Rc::new(|v: &[F]| vec![v[0].pow([2 as u64])]);
    //         let gate = Gatebb::new(2, 1, 1, f);
    //         let tmp = gate.cross_terms_adjust(&vec![F::ONE], &vec![F::ONE], d);
    //         println!("{:?}", tmp.iter().map(|v|v[0]).collect::<Vec<_>>());
    //     }
    // }
    
    #[test]
    
    fn test_circuit_builder() {
        let public_input_source = ExternalValue::<F>::new();
    
        let mut circuit = Circuit::<F, Gatebb<F>, Build>::new(2, 1);
    
        let sq = PolyOp::new(2, 1, 1, |x| vec!(x[0]*x[0]));
        let input = circuit.advice_pub(0, Advice::new(0, 1, 1, |_, iext| vec![iext[0]]), vec![], vec![&public_input_source])[0];
        let sq1 = circuit.apply(0, sq.clone(), vec![input]);
        let _ = circuit.apply_pub(0, sq.clone(), sq1);
    
        let mut circuit = circuit.finalize();
    
        public_input_source.set(F::from(2)).unwrap();
    
        circuit.execute(0);
    
        let var = Variable { visibility: Visibility::Public, round: 0, index: 2 };
        assert_eq!(F::from(16), circuit.cs.getvar(var));
    }
    
    #[test]
    
    fn test_permutation_argument() {
        let pi_ext : Vec<_> = repeat(ExternalValue::<F>::new()).take(5).collect();
        let challenge_ext = ExternalValue::<F>::new();
    
        let mut circuit = Circuit::new(2, 2);
        
        let one = circuit.one();
    
        let read_pi_advice = Advice::new(0, 1, 1, |_, iext: &[F]| vec![iext[0]]);
        
    
        let mut pi = vec![];
        for k in 0..5{
            pi.push(
                circuit.advice_pub(0, read_pi_advice.clone(), vec![], vec![&pi_ext[k]])[0]
            );
        }
    
        let challenge = circuit.advice_pub(1, read_pi_advice.clone(), vec![], vec![&challenge_ext])[0];
    
        let division_advice = Advice::new(2, 0, 1, |ivar : &[F], _| {
            let ch = ivar[0];
            let x = ivar[1];
            vec![(x-ch).invert().unwrap()]
        });
    
        let mut fractions = vec![];
        for k in 0..5 {
            fractions.push(
                circuit.advice(1, division_advice.clone(), vec![challenge, pi[k]], vec![])[0]
            );
        }
    
        let div_constr = Gatebb::<F>::new(2, 4, 1, Rc::new(|args|{
            let one = args[0];
            let ch = args[1];
            let x = args[2];
            let res = args[3];
            vec![one*one - res * (x-ch)]
        }));
    
        for k in 0..5 {
            circuit.constrain(&[one, challenge, pi[k], fractions[k]], div_constr.clone());
        }
    
        let mut circuit = circuit.finalize();
    
        // construction phase ended
    
        pi_ext[0].set(F::from(2)).unwrap();
        pi_ext[1].set(F::from(3)).unwrap();
        pi_ext[2].set(F::from(4)).unwrap();
        pi_ext[3].set(F::from(5)).unwrap();
        pi_ext[4].set(F::from(6)).unwrap();
    
        circuit.execute(0);
    
        challenge_ext.set(F::random(OsRng)).unwrap();
        circuit.execute(1);
    
        circuit.cs.valid_witness(); // test that constraints are satisfied
    }
    
    #[test]
    fn test_poseidon_gadget(){
        let cfg = Poseidon::new();
        let pi_ext = ExternalValue::<F>::new();
        let mut circuit = Circuit::new(25, 1);
        let read_pi_advice = Advice::new(0, 1, 1, |_, iext: &[F]| vec![iext[0]]);    
        let pi = circuit.advice_pub(0, read_pi_advice.clone(), vec![], vec![&pi_ext])[0];
        let ret = poseidon_gadget(&mut circuit, &cfg, 1, 0, vec![pi]);
    
        let mut circuit = circuit.finalize();
    
        pi_ext.set(F::ONE).unwrap();
    
        circuit.execute(0);
        circuit.cs.valid_witness();
    
        assert_eq!(circuit.cs.getvar(ret), F::from_str_vartime("18586133768512220936620570745912940619677854269274689475585506675881198879027").unwrap());
    }
    
    #[test]
    fn test_poseidon_gadget_k_equals_two(){
        let cfg = Poseidon::new();
        let pi_ext = ExternalValue::<F>::new();
        let mut circuit = Circuit::new(25, 1);
        let read_pi_advice = Advice::new(0, 1, 1, |_, iext: &[F]| vec![iext[0]]);    
        let pi = circuit.advice_pub(0, read_pi_advice.clone(), vec![], vec![&pi_ext])[0];
        let ret = poseidon_gadget(&mut circuit, &cfg, 2, 0, vec![pi]);
    
        let mut circuit = circuit.finalize();
    
        pi_ext.set(F::ONE).unwrap();
    
        circuit.execute(0);
        circuit.cs.valid_witness();
    
        assert_eq!(circuit.cs.getvar(ret), F::from_str_vartime("18586133768512220936620570745912940619677854269274689475585506675881198879027").unwrap());
    }

    #[test]
    fn test_poseidon_gadget_mixstrat(){
        let cfg = Poseidon::new();
        let pi_ext = ExternalValue::<F>::new();
        let mut circuit = Circuit::new(25, 1);
        let read_pi_advice = Advice::new(0, 1, 1, |_, iext: &[F]| vec![iext[0]]);    
        let pi = circuit.advice_pub(0, read_pi_advice.clone(), vec![], vec![&pi_ext])[0];
        let ret = poseidon_gadget_mixstrat(&mut circuit, &cfg, 0, vec![pi]);

        let mut circuit = circuit.finalize();

        pi_ext.set(F::ONE).unwrap();

        circuit.execute(0);
        circuit.cs.valid_witness();

        assert!(circuit.cs.getvar(ret) == F::from_str_vartime("18586133768512220936620570745912940619677854269274689475585506675881198879027").unwrap());

        println!("{:?}", circuit.cs.getvar(ret).to_repr());
    }

    #[test]
    
    fn test_bit_decomposition(){
        let pi_ext = ExternalValue::<F>::new();
        let mut circuit = Circuit::new(2, 1);
        let read_pi_advice = Advice::new(0, 1, 1, |_, iext: &[F]| vec![iext[0]]);    
        let pi = circuit.advice_pub(0, read_pi_advice.clone(), vec![], vec![&pi_ext])[0];
    
        let bits = bit_decomposition_gadget(&mut circuit, 0, 3, pi);
    
        let mut circuit = circuit.finalize();
        pi_ext.set(F::from(6)).unwrap();
        circuit.execute(0);
    
        circuit.cs.valid_witness();
    
        assert!(bits.len()==3);
        assert!(circuit.cs.getvar(bits[0]) == F::ZERO);
        assert!(circuit.cs.getvar(bits[1]) == F::ONE);
        assert!(circuit.cs.getvar(bits[2]) == F::ONE);
    }
    
    #[test]
    
    fn test_check_poly() {
        let f = Rc::new(|x: &[F]|{vec![x[0].pow([5])]});
        check_poly(5, 1, 1, f).unwrap();
    }

    
    #[test]
    
    fn test_scale(){
        let x = F::random(OsRng);
        for y in 0..100 {
            assert!(x.scale(y) == x*F::from(y));
        }
    }
    
    #[test]
    
    fn test_add() {
        let pt1 = C::random(OsRng).to_affine();
        let pt2 = C::random(OsRng).to_affine();
        
        let pt1_ = (pt1.x, pt1.y);
        let pt2_ = (pt2.x, pt2.y);
    
        let pt3_ = add_proj::<F,C>(pt1_, pt2_);
    
        let r3_inv = pt3_.2.invert().unwrap();
        let pt3 = grumpkin::G1Affine::from_xy(pt3_.0*r3_inv, pt3_.1*r3_inv).unwrap();
    
        assert!(Into::<C>::into(pt3) == pt1+pt2);
    }

    #[test]

    fn test_double() {
        let pt = C::random(OsRng).to_affine();
        let pt_ = (pt.x, pt.y);
        let pt2_ = double_proj::<F,C>(pt_);
        let rinv = pt2_.2.invert().unwrap();
        let pt2 = grumpkin::G1Affine::from_xy(pt2_.0*rinv, pt2_.1*rinv).unwrap();
        assert!(Into::<C>::into(pt2) == pt+pt);

    }

    #[test]

    fn test_range_check() {
        for range in 1..10 {
            for i in 1..10 {
                let x = F::from(i);
                assert!(if i < range {rangecheck(x, range) == F::ZERO} else {rangecheck(x, range) != F::ZERO});
            }
        }
    }

    #[test]

    fn test_limb_decompose_gadget() {

        let pi_ext = ExternalValue::<F>::new();
        let mut circuit = Circuit::new(9, 1);
        let read_pi_advice = Advice::new(0, 1, 1, |_, iext: &[F]| vec![iext[0]]);    
        let pi = circuit.advice_pub(0, read_pi_advice.clone(), vec![], vec![&pi_ext])[0];
    
        let limbs = limb_decompose_gadget(&mut circuit, 9, 0, 2, pi);
    
        let mut circuit = circuit.finalize();
        pi_ext.set(F::from(25)).unwrap();
        circuit.execute(0);
    
        circuit.cs.valid_witness();
    
        assert!(limbs.len()==2);
        assert!(circuit.cs.getvar(limbs[0].var) == F::from(7));
        assert!(circuit.cs.getvar(limbs[1].var) == F::from(2));

    }

    #[test]

    fn test_lagrange_choice() -> () {
        for n in 2..12 {
            for t in 0..n {
                assert!(find_degree(32, 1, 1, Rc::new(move |v: &[F]| vec![lagrange_choice(v[0],t,n)])).unwrap() == (n-1) as usize);
                for x in 0..n {
                    if x == t {
                        assert!(lagrange_choice(F::from(x), t, n) == F::ONE);
                    } else {
                        assert!(lagrange_choice(F::from(x), t, n) == F::ZERO);
                    }
                }

            }
        }
    }

    #[test]

    fn test_lagrange_batch() -> () {
        for n in 2..12 {
            assert!(find_degree(32, 1, n, Rc::new(move |v: &[F]| lagrange_choice_batched(v[0], n as u64))).unwrap() == (n-1));
            for x in 0..n {
                let v = lagrange_choice_batched(F::from(x as u64), n as u64);
                for t in 0..n {
                    assert!(if t==x {v[t] == F::ONE} else {v[t] == F::ZERO})
                }
            }
        }
    }

    #[test]

    fn test_choice_gadget() -> () {

        let mut pi_ext = vec![];
        let pi_id_ext = ExternalValue::<F>::new();
        for i in 0..9 {
            pi_ext.push(vec![]);
            for _ in 0..3 {
                pi_ext[i].push(ExternalValue::<F>::new());
            }
        }
        let mut circuit = Circuit::new(10, 1);
        let read_pi_advice = Advice::new(0, 1, 1, |_, iext: &[F]| vec![iext[0]]);    
        let mut pi = vec![];
        for i in 0..9 {
            pi.push(vec![]);
            for j in 0..3 {
                pi[i].push(circuit.advice_pub(0, read_pi_advice.clone(), vec![], vec![&pi_ext[i][j]])[0]);
            }
        }
        let pi_id = circuit.advice_pub(0, read_pi_advice, vec![], vec![&pi_id_ext])[0];
    
        let pi : Vec<_> = pi.iter().map(|x|x.as_ref()).collect();
        let chosen = choice_gadget(&mut circuit, &pi, VarSmall::new_unchecked(pi_id, 9), 0);

        let mut circuit = circuit.finalize();
        pi_id_ext.set(F::from(5)).unwrap();
        for i in 0..9 {
            for j in 0..3 {
                pi_ext[i][j].set(F::random(OsRng)).unwrap();
            }
        }
        circuit.execute(0);
    
        circuit.cs.valid_witness();

        assert!(chosen.len() == 3);

        for j in 0..3 {
            assert!(circuit.cs.getvar(pi[5][j]) == circuit.cs.getvar(chosen[j]))
        }
    }

    #[test]
    fn test_escalarmul_gadget()->(){
        let pi_a_ext = (ExternalValue::<F>::new(), ExternalValue::<F>::new());
        let pi_b_ext = (ExternalValue::<F>::new(), ExternalValue::<F>::new()); // a*(1+9+...+9^{nl-1})+b=0 must be checked out of band
        let pi_pt_ext = (ExternalValue::<F>::new(), ExternalValue::<F>::new());
        let pi_sc_ext = ExternalValue::<F>::new();

        let mut circuit = Circuit::new(10, 1);

        let read_pi = Advice::new(0,1,1, |_, iext: &[F]| vec![iext[0]]);    

        let x = circuit.advice(0, read_pi.clone(), vec![], vec![&pi_a_ext.0])[0];
        let y = circuit.advice(0, read_pi.clone(), vec![], vec![&pi_a_ext.1])[0];
        let a = EcAffinePoint::<F,C>::new(&mut circuit, x, y);
        let x = circuit.advice(0, read_pi.clone(), vec![], vec![&pi_b_ext.0])[0];
        let y = circuit.advice(0, read_pi.clone(), vec![], vec![&pi_b_ext.1])[0];
        let b = EcAffinePoint::<F,C>::new(&mut circuit, x, y);
        let x = circuit.advice(0, read_pi.clone(), vec![], vec![&pi_pt_ext.0])[0];
        let y = circuit.advice(0, read_pi.clone(), vec![], vec![&pi_pt_ext.1])[0];
        let pt = EcAffinePoint::<F,C>::new(&mut circuit, x, y);
        let sc = circuit.advice(0, read_pi.clone(), vec![], vec![&pi_sc_ext])[0];

        let mut nonzeros = vec![];
        let num_limbs = 81;

        let scmul = escalarmul_gadget_9(&mut circuit, sc, pt, num_limbs, 0, a, b, &mut nonzeros);

        nonzero_gadget(&mut circuit, &nonzeros, 9);
        let mut circuit = circuit.finalize();

        let pi_a = C::random(OsRng).to_affine();
        pi_a_ext.0.set(pi_a.x).unwrap(); pi_a_ext.1.set(pi_a.y).unwrap();

        //1+9+81+...+9^{num_limbs - 1} = (9^{num_limbs}-1)/8

        let bscale = (Fq::from(9).pow([num_limbs as u64])-Fq::ONE)*(Fq::from(8).invert().unwrap());
        let pi_b = -(C::from(pi_a)*bscale).to_affine();
        pi_b_ext.0.set(pi_b.x).unwrap(); pi_b_ext.1.set(pi_b.y).unwrap();

        let pi_pt = C::random(OsRng).to_affine();
        pi_pt_ext.0.set(pi_pt.x).unwrap(); pi_pt_ext.1.set(pi_pt.y).unwrap();

        pi_sc_ext.set(F::from(23)).unwrap();

        circuit.execute(0);

        circuit.cs.valid_witness();

        let answer = grumpkin::G1Affine::from_xy(circuit.cs.getvar(scmul.x), circuit.cs.getvar(scmul.y)).unwrap();

        assert!(answer == (pi_pt*<C as CurveExt>::ScalarExt::from(23)).to_affine());

        println!("Total circuit size: private: {} public: {}", circuit.cs.wtns[0].privs.len(), circuit.cs.wtns[0].pubs.len());
    }


}