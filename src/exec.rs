use typed_exec_graph::{self, exec_graph::{AnyData, Storage, Execution}, op};

fn f (x:i32,y:i32,z:i32)->(i32,i32,i32,i32){(x+y, x-y, x+2*y, x)}

#[test]
fn test() -> () {
    let mut arr = AnyData::new();
    let var = arr.alloc();
    let var2 = arr.alloc();
    let var3 = arr.alloc();
    arr.set::<i32>(2, var).unwrap();
    arr.set::<i32>(2, var2).unwrap();
    arr.set::<i32>(2, var3).unwrap();

    let mut arr = Execution { operations: vec![], storage: arr };

    let arr = &mut arr;
    op!(arr: out1, out2, out3, out4 <-- f(var, var2, var3));
    op!(arr: out5, out6, out7, out8 <-- f(var, out1, out2));
    arr.exec(0);
    arr.exec(1);
    let val = arr.storage().get::<i32>(out5).unwrap();
    println!("{}", val);
}