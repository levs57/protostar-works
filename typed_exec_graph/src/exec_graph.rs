use std::{marker::PhantomData, any::Any, cell::RefCell, rc::{Rc, Weak}, borrow::BorrowMut, sync::atomic::{AtomicU64, Ordering}};

static STORAGE_COUNTER : AtomicU64 = AtomicU64::new(0);


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

macro_rules! construct_operation {
    ($storage:ident: $first_out:ident $(,$output:ident)* <-- $func:ident($($input:ident),* $(,)?)) => {
        {
            let $first_out = $crate::exec_graph::Storage::alloc($storage);
            $(let $output = $crate::exec_graph::Storage::alloc($storage);)*
            let closure = move||{
                let ret_list = $crate::tuple_list::Tuple::into_tuple_list(
                    $func (
                        $( *::std::option::Option::unwrap($crate::exec_graph::Storage::get($storage, $input)),
                        )*

                    )
                );
                
                ::std::result::Result::unwrap($crate::exec_graph::Storage::set($storage, ret_list.0, $first_out));

                $(
                let ret_list = ret_list.1;
                ::std::result::Result::unwrap($crate::exec_graph::Storage::set($storage, ret_list.0, $output));
                )*
            };
            closure
        }
    }
}
#[test]

fn test() -> () {
    let mut arr = AnyData::new();
    let arr = &mut arr;
    let var = arr.alloc();
    let var2 = arr.alloc();
    let var3 = arr.alloc();
    arr.set::<u32>(2, var).unwrap();
    arr.set::<u32>(2, var2).unwrap();
    arr.set::<u32>(2, var3).unwrap();

    let val = arr.get::<u32>(var).unwrap();
    let f = |x:u32,y:u32,z:u32|{(x+y, x-y, x+2*y, x)};

    let mut closure = construct_operation!(arr: out1, out2, out3, out4 <-- f(var, var2, var3));
    closure();
    println!("{}", val);
}