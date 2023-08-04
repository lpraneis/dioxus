use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    marker::PhantomData,
    rc::Rc,
};

use bumpalo::Bump;

/// # Example
///
/// ```compile_fail
/// let data = String::from("hello world");
/// let store = Store::default();
/// let owner = store.owner();
/// let key = owner.insert(&data);
/// drop(data);
/// assert_eq!(*key.read(), "hello world");
/// ```
#[allow(unused)]
fn compile_fail() {}

#[test]
fn reused() {
    let store = Store::default();
    let first_ptr;
    {
        let owner = store.owner();
        first_ptr = owner.insert(1).raw.data.as_ptr();
        drop(owner);
    }
    {
        let owner = store.owner();
        let second_ptr = owner.insert(1234).raw.data.as_ptr();
        assert_eq!(first_ptr, second_ptr);
        drop(owner);
    }
}

#[test]
fn leaking_is_ok() {
    let data = String::from("hello world");
    let store = Store::default();
    let key;
    {
        // create an owner
        let owner = store.owner();
        // insert data into the store
        key = owner.insert(data);
        // don't drop the owner
        std::mem::forget(owner);
    }
    assert_eq!(key.try_read().as_deref(), Some(&"hello world".to_string()));
}

#[test]
fn drops() {
    let data = String::from("hello world");
    let store = Store::default();
    let key;
    {
        // create an owner
        let owner = store.owner();
        // insert data into the store
        key = owner.insert(data);
        // drop the owner
    }
    assert!(key.try_read().is_none());
}

#[test]
fn works() {
    let store = Store::default();
    let owner = store.owner();
    let key = owner.insert(1);

    assert_eq!(*key.read(), 1);
}

#[test]
fn insert_while_reading() {
    let store = Store::default();
    let owner = store.owner();
    let key;
    {
        let data: String = "hello world".to_string();
        key = owner.insert(data);
    }
    let value = key.read();
    owner.insert(&1);
    assert_eq!(*value, "hello world");
}

#[test]
#[should_panic]
fn panics() {
    let store = Store::default();
    let owner = store.owner();
    let key = owner.insert(1);
    drop(owner);

    assert_eq!(*key.read(), 1);
}

#[test]
fn fuzz() {
    fn maybe_owner_scope(
        store: &Store,
        valid_keys: &mut Vec<CopyHandle<String>>,
        invalid_keys: &mut Vec<CopyHandle<String>>,
        path: &mut Vec<u8>,
    ) {
        let branch_cutoff = 5;
        let children = if path.len() < branch_cutoff {
            rand::random::<u8>() % 4
        } else {
            rand::random::<u8>() % 2
        };

        for i in 0..children {
            let owner = store.owner();
            let key = owner.insert(format!("hello world {path:?}"));
            valid_keys.push(key);
            path.push(i);
            // read all keys
            println!("{:?}", path);
            for key in valid_keys.iter() {
                let value = key.read();
                println!("{:?}", value);
                assert!(value.starts_with("hello world"));
            }
            #[cfg(debug_assertions)]
            for key in invalid_keys.iter() {
                assert!(!key.validate());
            }
            maybe_owner_scope(store, valid_keys, invalid_keys, path);
            invalid_keys.push(valid_keys.pop().unwrap());
            path.pop();
        }
    }

    for _ in 0..10 {
        let store = Store::default();
        maybe_owner_scope(&store, &mut Vec::new(), &mut Vec::new(), &mut Vec::new());
    }
}

pub struct CopyHandle<T> {
    raw: MemoryLocation,
    #[cfg(debug_assertions)]
    generation: u32,
    _marker: PhantomData<T>,
}

impl<T: 'static> CopyHandle<T> {
    #[inline(always)]
    fn validate(&self) -> bool {
        #[cfg(debug_assertions)]
        {
            self.raw.generation.get() == self.generation
        }
        #[cfg(not(debug_assertions))]
        {
            true
        }
    }

    pub fn try_read(&self) -> Option<Ref<'_, T>> {
        self.validate()
            .then(|| {
                Ref::filter_map(self.raw.data.borrow(), |any| {
                    any.as_ref()?.downcast_ref::<T>()
                })
                .ok()
            })
            .flatten()
    }

    pub fn read(&self) -> Ref<'_, T> {
        self.try_read().unwrap()
    }

    pub fn try_write(&self) -> Option<RefMut<'_, T>> {
        self.validate()
            .then(|| {
                RefMut::filter_map(self.raw.data.borrow_mut(), |any| {
                    any.as_mut()?.downcast_mut::<T>()
                })
                .ok()
            })
            .flatten()
    }

    pub fn write(&self) -> RefMut<'_, T> {
        self.try_write().unwrap()
    }
}

impl<T> Copy for CopyHandle<T> {}

impl<T> Clone for CopyHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Clone, Copy)]
struct MemoryLocation {
    data: &'static RefCell<Option<Box<dyn std::any::Any>>>,
    #[cfg(debug_assertions)]
    generation: &'static Cell<u32>,
}

impl MemoryLocation {
    #[allow(unused)]
    fn drop(&self) {
        let old = self.data.borrow_mut().take();
        #[cfg(debug_assertions)]
        if old.is_some() {
            let new_generation = self.generation.get() + 1;
            self.generation.set(new_generation);
        }
    }

    fn replace<T: 'static>(&mut self, value: T) -> CopyHandle<T> {
        let mut inner_mut = self.data.borrow_mut();

        let raw = Box::new(value);
        let old = inner_mut.replace(raw);
        assert!(old.is_none());
        CopyHandle {
            raw: *self,
            #[cfg(debug_assertions)]
            generation: self.generation.get(),
            _marker: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct Store {
    bump: &'static Bump,
    recycled: Rc<RefCell<Vec<MemoryLocation>>>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            bump: Box::leak(Box::new(Bump::new())),
            recycled: Default::default(),
        }
    }
}

impl Store {
    fn recycle(&self, location: MemoryLocation) {
        location.drop();
        self.recycled.borrow_mut().push(location);
    }

    fn claim(&self) -> MemoryLocation {
        if let Some(location) = self.recycled.borrow_mut().pop() {
            location
        } else {
            let data: &'static RefCell<_> = self.bump.alloc(RefCell::new(None));
            MemoryLocation {
                data,
                #[cfg(debug_assertions)]
                generation: self.bump.alloc(Cell::new(0)),
            }
        }
    }

    pub fn owner(&self) -> Owner {
        Owner {
            store: self.clone(),
            owned: Default::default(),
        }
    }
}

pub struct Owner {
    store: Store,
    owned: Rc<RefCell<Vec<MemoryLocation>>>,
}

impl Owner {
    pub fn insert<T: 'static>(&self, value: T) -> CopyHandle<T> {
        let mut location = self.store.claim();
        let key = location.replace(value);
        self.owned.borrow_mut().push(location);
        key
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        for location in self.owned.borrow().iter() {
            self.store.recycle(*location)
        }
    }
}