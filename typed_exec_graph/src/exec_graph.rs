use std::{marker::PhantomData, any::Any, cell::RefCell, rc::{Rc, Weak}, borrow::BorrowMut, sync::atomic::{AtomicU64, Ordering}};

pub static STORAGE_COUNTER : AtomicU64 = AtomicU64::new(0);

#[derive(Copy, Clone)]
pub struct Var<T>{
    addr: usize,
    storage: u64,
    _marker: PhantomData<T>,
}

pub trait Storage {
    fn new() -> Self;
    fn alloc<T: 'static>(&mut self) -> Var<T>;
    fn get<T: 'static>(&self, addr: Var<T>) -> Option<Rc<T>>;
    fn set<T: 'static >(&mut self, value: T, addr: Var<T>) -> Result<(), String>;
    fn replace<T: 'static >(&mut self, value: T, addr: Var<T>) -> ();
    fn uid(&self) -> u64;
}

pub struct AnyData {
    data: Vec<Option<Rc<dyn Any>>>,
    uid: u64,
}

impl Storage for AnyData {
    fn new() -> Self {
        Self {data: vec![], uid: STORAGE_COUNTER.fetch_add(1, Ordering::Relaxed) }
    }

    fn alloc<T: 'static>(&mut self) -> Var<T> {
        self.data.push(None);
        Var{addr: self.data.len()-1, storage: self.uid, _marker: PhantomData}
    }

    fn get<T: 'static>(&self, addr: Var<T>) -> Option<Rc<T>> {
        debug_assert!(addr.storage == self.uid);
        let addr = addr.addr;
        match &self.data[addr] {
            None => None,
            Some(value) => Some(value.clone().downcast().unwrap()),
        }
    }

    fn set<T: 'static >(&mut self, value: T, addr: Var<T>) -> Result<(), String> {
        debug_assert!(addr.storage == self.uid);
        let addr = addr.addr;
        match self.data[addr] {
            Some(_) => Err(String::from("The value is already set.")),
            None => {self.data[addr] = Some(Rc::new(value)); Ok(())}
        }
    }

    fn replace<T: 'static >(&mut self, value: T, addr: Var<T>) -> () {
        debug_assert!(addr.storage == self.uid);
        let addr = addr.addr;
        let tmp = (self.data[addr].as_mut()).unwrap();
        let stored : &mut T = Rc::get_mut(tmp)
            .unwrap()
            .downcast_mut()
            .unwrap();
        *stored = value;        
    }

    fn uid(&self) -> u64 {
        self.uid
    }
}


pub fn construct_operation_simple<Inp: 'static, Out: 'static, Stor: Storage, Fun: Fn(&Inp)->Out + 'static>
(
    inp: Var<Inp>,
    out: Var<Out>,
    f: Fun,
) -> Box<dyn FnOnce(&mut Stor)->()> {
    
    let closure = move |st: &mut Stor| {
        let in_value = st.get(inp).unwrap();
        let out_value = f(&in_value);
        st.set(out_value, out).unwrap();
    };

    Box::new(closure)
}

pub struct Execution<Stor: Storage> {
    pub operations: Vec<Rc<dyn Fn(&mut Stor) -> ()>>,
    pub storage: Stor,
}

impl<Stor: Storage> Execution<Stor> {
    pub fn storage(&mut self) -> &mut Stor {
        &mut self.storage
    }

    pub fn op_push (&mut self, s: Rc<dyn Fn(&mut Stor) -> ()>) -> () {
        self.operations.push(s);
    }

    pub fn exec(&mut self, i: usize) -> () {
        let op = self.operations[i].clone();
        op(self.storage());
    }
}

#[macro_export]
macro_rules! op {
    ($execution:ident : $first_out:ident $(,$output:ident)* <-- $func:ident($($input:ident),* $(,)?)) =>
    
    {
         let $first_out = $crate::exec_graph::Storage::alloc(
             $crate::exec_graph::Execution::storage($execution)
         );
         $(let $output = $crate::exec_graph::Storage::alloc(
             $crate::exec_graph::Execution::storage($execution)
         );)*
        let tmp = 
            move |st: &mut _|{
                let ret_list = $crate::tuple_list::Tuple::into_tuple_list(
                    $func (
                        $( *::std::option::Option::unwrap($crate::exec_graph::Storage::get(st, $input)),
                        )*

                    )
                );

                ::std::result::Result::unwrap($crate::exec_graph::Storage::set(st, ret_list.0, $first_out));

                $(
                let ret_list = ret_list.1;
                ::std::result::Result::unwrap($crate::exec_graph::Storage::set(st, ret_list.0, $output));
                )*
            };
        

        let tmp = ::std::rc::Rc::new(tmp);
        
        $execution.op_push(tmp);
        
    }
}

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
    //op!(brr: out5, out6, out7, out8 <-- f(var, out1, out2));
    arr.exec(0);
    arr.exec(1);
    let val = arr.storage().get::<i32>(out5).unwrap();
    println!("{}", val);
}