use std::{cell::RefCell, rc::Rc};

pub struct RunAllocator {
    free_slots: Vec<usize>,
    total_spawned: usize,
    active: usize,
    allocated: usize,
}

impl RunAllocator {
    pub fn new() -> Self {
        Self { free_slots: vec![], total_spawned: 0, active: 0, allocated: 0 }
    }

    pub fn allocate(&mut self) -> RunIndex {
        let slot = self.free_slots.pop().unwrap_or_else(|| {
            self.allocated += 1;
            self.allocated - 1
        });
        self.active += 1;
        self.total_spawned += 1;
        RunIndex { muid: self.total_spawned - 1, slot }
    }
    
    pub fn deallocate(&mut self, idx: RunIndex) {
        self.free_slots.push(idx.slot);
        self.active -= 1;
    }
}

#[derive(Debug)]
pub struct RunIndex {
    pub(crate) muid: usize,
    pub(crate) slot: usize,
}

struct CuratedSlot<T> {
    value: T,
    muid: usize,
}

#[derive(Clone)]
pub struct InnerValue<T: Clone> {
    data: Rc<RefCell<Vec<Option<CuratedSlot<T>>>>>
}

impl<T: Clone> InnerValue<T> {
    pub fn new() -> Self {
        Self {
            data: Rc::new(RefCell::new(vec![])),
        }
    }

    pub fn get(&self, idx: &RunIndex) -> Option<T> {
        match self.data.borrow().get(idx.slot) {
            Some(opt) => match opt {
                Some(CuratedSlot{value, muid}) => match muid == &idx.muid {
                    true => Some(value.clone()),
                    false => match muid < &idx.muid {
                        true => None,
                        false => panic!("Accessing value with muid {} by idx: {:?}", muid, idx),
                    },
                },
                None => None,
            },
            None => None,
        }

    }

    pub fn set(&self, idx: &RunIndex, value: T) {
        assert!(self.replace(idx, value).is_none(), "Attempting to set value by idx: {:?}, but is already set", idx)
    }

    #[must_use = "if you intended to set a value, consider `set` method instead"]
    pub fn replace(&self, idx: &RunIndex, value: T) -> Option<T> {
        let mut data = self.data.borrow_mut();
        if data.len() <= idx.slot {
            data.resize_with(idx.slot + 1, || None);
        }
        match data[idx.slot] {
            Some(CuratedSlot{muid, value: _}) => assert!(muid <= idx.muid, "Writing value with muid {} by idx: {:?}", muid, idx),
            None => (),
        }
        data[idx.slot].replace(CuratedSlot { value: value.clone(), muid: idx.muid }).and_then(|CuratedSlot{value, muid}| if muid == idx.muid {Some(value)} else {None})
    }
}


#[test]
fn normal_usage() {
    let x = InnerValue::new();

    x.set(&RunIndex { muid: 0, slot: 0 }, 0);
    x.set(&RunIndex { muid: 1, slot: 1 }, 1);
    x.set(&RunIndex { muid: 2, slot: 2 }, 2);
    assert_eq!(x.get(&RunIndex { muid: 0, slot: 0 }), Some(0));
    assert_eq!(x.get(&RunIndex { muid: 1, slot: 1 }), Some(1));
    assert_eq!(x.get(&RunIndex { muid: 2, slot: 2 }), Some(2));
    assert_eq!(x.get(&RunIndex { muid: 3, slot: 0 }), None);
    x.set(&RunIndex { muid: 3, slot: 0 }, 3);
    assert_eq!(x.get(&RunIndex { muid: 3, slot: 0 }), Some(3));
    assert_eq!(x.replace(&RunIndex { muid: 1, slot: 1 }, 5), Some(1))
}

