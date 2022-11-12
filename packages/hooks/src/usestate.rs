#![warn(clippy::pedantic)]

use dioxus_core::{prelude::*, UpdateScope};
use std::{
    cell::RefMut,
    collections::VecDeque,
    fmt::{Debug, Display},
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::{Add, Deref, DerefMut, Div, Mul, Not, Sub},
    rc::Rc,
    sync::Arc,
};

/// Store state between component renders.
///
/// ## Dioxus equivalent of useState, designed for Rust
///
/// The Dioxus version of `useState` for state management inside components. It allows you to ergonomically store and
/// modify state between component renders. When the state is updated, the component will re-render.
///
///
/// ```ignore
/// const Example: Component = |cx| {
///     let count = use_state(&cx, || 0);
///
///     cx.render(rsx! {
///         div {
///             h1 { "Count: {count}" }
///             button { onclick: move |_| *count.modify() += 1, "Increment" }
///             button { onclick: move |_| *count.modify() -= 1, "Decrement" }
///         }
///     ))
/// }
/// ```
pub fn use_state<'a, T: 'static, Y>(
    cx: Scope<'a, Y>,
    initial_state_fn: impl FnOnce() -> T,
) -> &mut UseState<'a, T> {
    let hook = cx.use_hook(move || {
        struct DropHandler<'a, T: 'static>(UseState<'a, T>);

        impl<T> Drop for DropHandler<'_, T> {
            fn drop(&mut self) {
                unsafe {
                    ManuallyDrop::drop(&mut *self.0.value.0);
                }
            }
        }

        let current_val = initial_state_fn();
        let update_scope = cx.schedule_update_non_sync();

        let value = CopyCell::new(current_val);

        DropHandler(UseState {
            value,
            update_scope,
        })
    });

    &mut hook.0
}

pub struct UseState<'a, T: 'static> {
    pub(crate) update_scope: UpdateScope<'a>,
    pub(crate) value: CopyCell<'a, T>,
}

impl<T: 'static> UseState<'_, T> {
    /// Set the state to a new value.
    pub fn set(&self, new: T) {
        *self.value.borrow_mut() = new;
        self.needs_update();
    }

    /// Get the current value of the state by cloning its container Rc.
    ///
    /// This is useful when you are dealing with state in async contexts but need
    /// to know the current value. You are not given a reference to the state.
    ///
    /// # Examples
    /// An async context might need to know the current value:
    ///
    /// ```rust, ignore
    /// fn component(cx: Scope) -> Element {
    ///     let count = use_state(&cx, || 0);
    ///     cx.spawn({
    ///         let set_count = count.to_owned();
    ///         async move {
    ///             let current = set_count.current();
    ///         }
    ///     })
    /// }
    /// ```
    #[must_use]
    pub fn current(&self) -> RcCellBorrow<T> {
        self.value.borrow()
    }

    /// Set the state to a new value, using the current state value as a reference.
    ///
    /// This is similar to passing a closure to React's `set_value` function.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```rust, ignore
    /// # use dioxus_core::prelude::*;
    /// # use dioxus_hooks::*;
    /// fn component(cx: Scope) -> Element {
    ///     let value = use_state(&cx, || 0);
    ///
    ///     // to increment the value
    ///     value.modify(|v| v + 1);
    ///
    ///     // usage in async
    ///     cx.spawn({
    ///         let value = value.to_owned();
    ///         async move {
    ///             value.modify(|v| v + 1);
    ///         }
    ///     });
    ///
    ///     # todo!()
    /// }
    /// ```
    pub fn modify(&self, f: impl FnOnce(&T) -> T) {
        let new_val = {
            let current = self.value.borrow();
            f(&*current)
        };
        self.set(new_val);
    }

    /// Mark the component that create this [`UseState`] as dirty, forcing it to re-render.
    ///
    /// ```rust, ignore
    /// fn component(cx: Scope) -> Element {
    ///     let count = use_state(&cx, || 0);
    ///     cx.spawn({
    ///         let count = count.to_owned();
    ///         async move {
    ///             // for the component to re-render
    ///             count.needs_update();
    ///         }
    ///     })
    /// }
    /// ```
    pub fn needs_update(&self) {
        let mut sender = self.update_scope;
        sender.send();
    }

    pub fn borrow(&self) -> RcCellBorrow<'_, T> {
        self.value.borrow()
    }

    pub fn borrow_mut(&self) -> RcCellBorrowMut<'_, T> {
        self.value.borrow_mut()
    }

    /// Get a mutable handle to the value by calling `ToOwned::to_owned` on the
    /// current value.
    ///
    /// This is essentially cloning the underlying value and then setting it,
    /// giving you a mutable handle in the process. This method is intended for
    /// types that are cheaply cloneable.
    ///
    /// If you are comfortable dealing with `RefMut`, then you can use `make_mut` to get
    /// the underlying slot. However, be careful with `RefMut` since you might panic
    /// if the `RefCell` is left open.
    ///
    /// # Examples
    ///
    /// ```rust, ignore
    /// let val = use_state(&cx, || 0);
    ///
    /// val.with_mut(|v| *v = 1);
    /// ```
    pub fn with_mut(&self, apply: impl FnOnce(&mut T)) {
        apply(&mut self.value.borrow_mut());

        self.needs_update();
    }
}

impl<T> Clone for UseState<'_, T> {
    fn clone(&self) -> Self {
        Self {
            update_scope: self.update_scope,
            value: self.value,
        }
    }
}
impl<T> Copy for UseState<'_, T> {}

impl<T: 'static + Display> std::fmt::Display for UseState<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &*self.value.borrow())
    }
}

impl<T: std::fmt::Binary> std::fmt::Binary for UseState<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:b}", self.value.borrow().deref())
    }
}

impl<T: PartialEq> PartialEq<T> for UseState<'_, T> {
    fn eq(&self, other: &T) -> bool {
        *self.value.borrow().deref() == *other
    }
}

// todo: this but for more interesting conrete types
impl PartialEq<bool> for &UseState<'_, bool> {
    fn eq(&self, other: &bool) -> bool {
        *self.value.borrow().deref() == *other
    }
}

impl<T: PartialEq> PartialEq<UseState<'_, T>> for UseState<'_, T> {
    fn eq(&self, other: &UseState<'_, T>) -> bool {
        *self.value.borrow() == *other.value.borrow()
    }
}

impl<T: Debug> Debug for UseState<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.value.borrow().deref())
    }
}

impl<T: Not + Copy> std::ops::Not for &UseState<'_, T> {
    type Output = <T as std::ops::Not>::Output;

    fn not(self) -> Self::Output {
        self.value.borrow().deref().not()
    }
}

impl<T: Not + Copy> std::ops::Not for UseState<'_, T> {
    type Output = <T as std::ops::Not>::Output;

    fn not(self) -> Self::Output {
        self.value.borrow().deref().not()
    }
}

impl<T: std::ops::Add + Copy> std::ops::Add<T> for &UseState<'_, T> {
    type Output = <T as std::ops::Add>::Output;

    fn add(self, other: T) -> Self::Output {
        *self.value.borrow().deref() + other
    }
}
impl<T: std::ops::Sub + Copy> std::ops::Sub<T> for &UseState<'_, T> {
    type Output = <T as std::ops::Sub>::Output;

    fn sub(self, other: T) -> Self::Output {
        *self.value.borrow().deref() - other
    }
}

impl<T: std::ops::Div + Copy> std::ops::Div<T> for &UseState<'_, T> {
    type Output = <T as std::ops::Div>::Output;

    fn div(self, other: T) -> Self::Output {
        *self.value.borrow().deref() / other
    }
}

impl<T: std::ops::Mul + Copy> std::ops::Mul<T> for &UseState<'_, T> {
    type Output = <T as std::ops::Mul>::Output;

    fn mul(self, other: T) -> Self::Output {
        *self.value.borrow().deref() * other
    }
}

impl<T: Add<Output = T> + Copy> std::ops::AddAssign<T> for &UseState<'_, T> {
    fn add_assign(&mut self, rhs: T) {
        self.set((*self.current()) + rhs);
    }
}

impl<T: Sub<Output = T> + Copy> std::ops::SubAssign<T> for &UseState<'_, T> {
    fn sub_assign(&mut self, rhs: T) {
        self.set((*self.current()) - rhs);
    }
}

impl<T: Mul<Output = T> + Copy> std::ops::MulAssign<T> for &UseState<'_, T> {
    fn mul_assign(&mut self, rhs: T) {
        self.set((*self.current()) * rhs);
    }
}

impl<T: Div<Output = T> + Copy> std::ops::DivAssign<T> for &UseState<'_, T> {
    fn div_assign(&mut self, rhs: T) {
        self.set((*self.current()) / rhs);
    }
}

impl<T: Add<Output = T> + Copy> std::ops::AddAssign<T> for UseState<'_, T> {
    fn add_assign(&mut self, rhs: T) {
        self.set((*self.current()) + rhs);
    }
}

impl<T: Sub<Output = T> + Copy> std::ops::SubAssign<T> for UseState<'_, T> {
    fn sub_assign(&mut self, rhs: T) {
        self.set((*self.current()) - rhs);
    }
}

impl<T: Mul<Output = T> + Copy> std::ops::MulAssign<T> for UseState<'_, T> {
    fn mul_assign(&mut self, rhs: T) {
        self.set((*self.current()) * rhs);
    }
}

impl<T: Div<Output = T> + Copy> std::ops::DivAssign<T> for UseState<'_, T> {
    fn div_assign(&mut self, rhs: T) {
        self.set((*self.current()) / rhs);
    }
}

#[test]
fn api_makes_sense() {
    #[allow(unused)]
    fn callback_like<'a, T>(scope: Scope<'a, T>, _: impl FnOnce() + 'a) {}

    #[allow(unused)]
    fn app(cx: Scope) -> Element {
        let val = use_state(cx, || 0);

        val.set(0);
        val.modify(|v| v + 1);

        match *val.borrow() {
            10 => {
                val.set(20);
                val.modify(|v| v + 1);
            }
            20 => {}
            _ => {
                println!("{val}");
            }
        }

        callback_like(cx, || {
            val.set(0);
            val.modify(|v| v + 1);
        });
        callback_like(cx, || {
            val.borrow();
        });
        callback_like(cx, || {
            val.borrow_mut();
        });

        // cx.render(LazyNodes::new(|f| f.static_text("asd")))

        todo!()
    }
}

#[derive(Debug)]
struct RcCellInner<T> {
    refrenced: bool,
    data: T,
}

#[derive(Debug)]
pub struct CopyCell<'a, T>(*mut ManuallyDrop<RcCellInner<T>>, PhantomData<&'a ()>);

impl<'a, T> Clone for CopyCell<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}
impl<'a, T> Copy for CopyCell<'a, T> {}

impl<'a, T> CopyCell<'a, T> {
    fn new(data: T) -> Self {
        Self(
            &mut *Box::new(ManuallyDrop::new(RcCellInner {
                refrenced: false,
                data,
            })),
            PhantomData,
        )
    }

    fn borrow(&self) -> RcCellBorrow<'_, T> {
        unsafe {
            if (*self.0).refrenced {
                panic!("already borrowed");
            }
            (*self.0).refrenced = true;
        }
        RcCellBorrow(self.0, PhantomData)
    }

    fn borrow_mut(&self) -> RcCellBorrowMut<'_, T> {
        unsafe {
            if (*self.0).refrenced {
                panic!("already borrowed");
            }
            (*self.0).refrenced = true;
        }
        RcCellBorrowMut(self.0, PhantomData)
    }
}

pub struct RcCellBorrow<'a, T>(*mut ManuallyDrop<RcCellInner<T>>, PhantomData<&'a ()>);

impl<'a, T> Drop for RcCellBorrow<'a, T> {
    fn drop(&mut self) {
        unsafe {
            (*self.0).refrenced = false;
        }
    }
}

impl<'a, T> Deref for RcCellBorrow<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &(*self.0).data }
    }
}

pub struct RcCellBorrowMut<'a, T>(*mut ManuallyDrop<RcCellInner<T>>, PhantomData<&'a ()>);

impl<'a, T> Drop for RcCellBorrowMut<'a, T> {
    fn drop(&mut self) {
        unsafe {
            (*self.0).refrenced = false;
        }
    }
}

impl<'a, T> Deref for RcCellBorrowMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &(*self.0).data }
    }
}

impl<'a, T> DerefMut for RcCellBorrowMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut (*self.0).data }
    }
}

// // this should fail
// #[test]
// fn rc_cell_test_fail() {
//     fn create_with_lifetime<'a>(_: &'a ()) -> CopyCell<'a, i32> {
//         CopyCell::new(0)
//     }
//     fn in_lifetime<'a, 'b>(marker: &'a ()) -> CopyCell<'b, i32> {
//         create_with_lifetime(marker)
//     }
//     let _ = in_lifetime(&());
// }

#[test]
fn rc_cell_test() {
    let r = CopyCell::new(0);
    let x = r;
    let f = || {
        println!("{:?}", &*x.borrow());
    };
    let f2 = || {
        let mut_ref: &mut i32 = &mut x.borrow_mut();
        *mut_ref += 1;
        println!("{:?}", mut_ref);
    };
    println!("{:?}", { &mut *x.borrow_mut() });
    f2();
    f();
    println!("{:?}, {:?}", r, x);
    unsafe { r.0.drop_in_place() }
}

#[test]
fn sizes() {
    dbg!(std::mem::size_of::<CopyCell<'_, i32>>());
    dbg!(std::mem::size_of::<UseState<i32>>());
}

#[derive(Debug)]
struct Messages<T> {
    messages: VecDeque<T>,
}

impl<T> Default for Messages<T> {
    fn default() -> Self {
        Self {
            messages: VecDeque::new(),
        }
    }
}

impl<T> Messages<T> {
    fn push(&self, message: T) {
        unsafe {
            let raw: *const _ = &self.messages;
            let raw_mut = raw as *mut Messages<T>;
            (*raw_mut).messages.push_back(message);
        }
    }

    fn pop(&self) -> Option<T> {
        unsafe {
            let raw: *const _ = &self.messages;
            let raw_mut = raw as *mut Messages<T>;
            (*raw_mut).messages.pop_front()
        }
    }
}

pub struct Receiver<T> {
    messages: Box<Messages<T>>,
}

impl<T> Default for Receiver<T> {
    fn default() -> Receiver<T> {
        Receiver {
            messages: Box::new(Messages::default()),
        }
    }
}

impl<T: Debug> Receiver<T> {
    pub fn receive(&self) -> Option<T> {
        self.messages.pop()
    }

    pub fn sender(&self) -> Sender<T> {
        let raw: *const _ = &*self.messages;
        let raw_mut = raw as *mut Messages<T>;
        Sender {
            messages: raw_mut,
            l: PhantomData,
        }
    }
}

pub struct Sender<'a, T> {
    messages: *mut Messages<T>,
    l: PhantomData<&'a ()>,
}

impl<'a, T> Clone for Sender<'a, T> {
    fn clone(&self) -> Self {
        Self {
            messages: self.messages,
            l: self.l,
        }
    }
}
impl<'a, T> Copy for Sender<'a, T> {}

impl<'a, T: Debug> Sender<'a, T> {
    pub fn send(&mut self, message: T) {
        unsafe {
            (*self.messages).push(message);
        }
    }
}

#[test]
fn test() {
    let r = Receiver::default();
    let mut s = r.sender();
    for i in 0..100 {
        s.send(i);
    }
    for i in 0..100 {
        assert_eq!(r.receive(), Some(i));
    }
    assert_eq!(r.receive(), None);
}
