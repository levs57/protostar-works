use std::{fs::File, io::Write};

use ff::{PrimeField, Field};
use halo2::halo2curves::{bn256, serde::SerdeObject};
use num_traits::{pow, abs};

type F = bn256::Fr;

// fn precompute_roots_of_unity() -> (){
//     let mut s : String = "".to_string();
//     s+="use halo2::halo2curves::{bn256::Fr as F, serde::SerdeObject};\n";
//     s+="pub fn roots_of_unity(power: u64, logorder: usize) -> F {\n";
//     s+="  match logorder {\n";
//     for logorder in 0..10 {
//         s+=&format!("  {} => \n", logorder);
//         s+="    match power {\n";
//         for power in 0..pow(2, logorder) {
//             s+=&format!("       {} => F::from_raw_bytes_unchecked(&{:?}),\n", power, F::ROOT_OF_UNITY.pow([pow(2, F::S as usize - logorder)]).pow([power]).to_raw_bytes())
//         }
//         s+="        _ => panic!(),\n";
//         s+="    },\n";
//     }
//     s+="    _ => panic!(),\n";
//     s+="  }\n";
//     s+="}\n";

//     let mut f = File::create("./src/utils/powers_of_omega.rs").expect("Unable to create file");
//     f.write_all(s.as_bytes()).expect("Unable to write data");
// }

pub fn precompute_half_squares(output: &mut dyn Write, limit: u64) -> (){
    let mut s : String = "".to_string();
    s+="use halo2::halo2curves::{bn256::Fr as F, serde::SerdeObject};\n";
    s+="pub fn half_square(k:u64) -> F {\n";
    s+="    match k {\n";
    for i in 0..limit {
    s+=&format!("       {} => F::from_raw_bytes_unchecked(&{:?}),\n", i, (F::TWO_INV * F::from(i)).square().to_raw_bytes());
    }
    s+="        _ => panic!(),\n";
    s+="    }\n";
    s+="}\n";
    output.write_all(s.as_bytes()).expect("Unable to write data");
}

fn felt_from_i64(x: i64) -> F {
    let is_neg = x<0;
    let mut ret = F::from(abs(x) as u64);
    if is_neg {ret = -ret}
    ret
}

fn inv_lagrange_prod(k: u64, n: u64) -> F {
    assert!(k<n);
    let mut ret = F::ONE;
    for i in 0..n{
        if i != k {ret *= felt_from_i64(k as i64 - i as i64)}
    }
    ret.invert().unwrap()
}

pub fn precompute_inv_lagrange_prod(output: &mut dyn Write, limit: u64) -> () {
    let mut s : String = "".to_string();
    s+="use halo2::halo2curves::{bn256::Fr as F, serde::SerdeObject};\n";
    s+="pub fn inv_lagrange_prod(k: u64, n: u64) -> F {\n";
    s+="    match (k, n) {\n";
    for n in 2..limit {
    for k in 0..n {
    s+=&format!("        ({},{}) => F::from_raw_bytes_unchecked(&{:?}),\n", k, n, inv_lagrange_prod(k, n).to_raw_bytes());
    }
    }
    s+="        _ => panic!(),\n";
    s+="    }\n";
    s+="}\n";
    output.write_all(s.as_bytes()).expect("Unable to write data");
}
