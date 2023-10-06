use std::{fs::File, io::Write};

use ff::{PrimeField, Field};
use halo2curves::{bn256, serde::SerdeObject};
use num_traits::{pow, abs};

use crate::utils::field_precomp::FieldUtils;

type F = bn256::Fr;

// #[test]
fn precompute_roots_of_unity() -> (){
    let mut s : String = "".to_string();
    s+="use halo2curves::{bn256::Fr as F, serde::SerdeObject};\n";
    s+="pub fn roots_of_unity(power: u64, logorder: usize) -> F {\n";
    s+="  match logorder {\n";
    for logorder in 0..10 {
        s+=&format!("  {} => \n", logorder);
        s+="    match power {\n";
        for power in 0..pow(2, logorder) {
            s+=&format!("       {} => F::from_raw_bytes_unchecked(&{:?}),\n", power, F::ROOT_OF_UNITY.pow([pow(2, F::S as usize - logorder)]).pow([power]).to_raw_bytes())
        }
        s+="        _ => panic!(),\n";
        s+="    },\n";
    }
    s+="    _ => panic!(),\n";
    s+="  }\n";
    s+="}\n";

    let mut f = File::create("./src/utils/powers_of_omega.rs").expect("Unable to create file");
    f.write_all(s.as_bytes()).expect("Unable to write data");
}

// #[test]
fn precompute_half_squares() -> (){
    let mut s : String = "".to_string();
    s+="use halo2curves::{bn256::Fr as F, serde::SerdeObject};\n";
    s+="pub fn half_square(k:u64) -> F {\n";
    s+="    match k {\n";
    for i in 0..50 {
    s+=&format!("       {} => F::from_raw_bytes_unchecked(&{:?}),\n", i, F::TWO_INV.scale(i).square().to_raw_bytes());
    }
    s+="        _ => panic!(),\n";
    s+="    }\n";
    s+="}\n";
    let mut f = File::create("./src/utils/half_squares.rs").expect("Unable to create file");
    f.write_all(s.as_bytes()).expect("Unable to write data");
}

fn inv_lagrange_prod(k: u64, n: u64) -> F {
    assert!(k<n);
    let mut acc : i64 = 1;
    for i in 0..n{
        if i != k {acc *= k as i64 - i as i64}
    }
    let is_neg = acc<0;

    let mut ret = F::from(abs(acc) as u64).invert().unwrap();
    if is_neg {ret = -ret}

    ret
}

// #[test]
fn precompute_inv_lagrange_prod() -> () {
    let mut s : String = "".to_string();
    s+="use halo2curves::{bn256::Fr as F, serde::SerdeObject};\n";
    s+="pub fn inv_lagrange_prod(k: u64, n: u64) -> F {\n";
    s+="    match (k, n) {\n";
    for n in 2..20 {
    for k in 0..n {
    s+=&format!("        ({},{}) => F::from_raw_bytes_unchecked(&{:?}),\n", k, n, inv_lagrange_prod(k, n).to_raw_bytes());
    }
    }
    s+="        _ => panic!(),\n";
    s+="    }\n";
    s+="}\n";
    let mut f = File::create("./src/utils/inv_lagrange_prod.rs").expect("Unable to create file");
    f.write_all(s.as_bytes()).expect("Unable to write data");
}