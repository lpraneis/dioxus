#![warn(clippy::pedantic)]

use dioxus_core::{prelude::*, UpdateScope};
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::VecDeque,
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::{Add, AddAssign, Deref, DerefMut, Div, DivAssign, Mul, MulAssign, Not, Sub, SubAssign},
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
) -> UseState<'a, T, BorrowedRcCell<'a, T>> {
    let hook = cx.use_hook(move || {
        let current_val = initial_state_fn();
        let update_scope = cx.schedule_update_non_sync();

        let value = RcCell::new(current_val);

        UseState {
            value,
            update_scope,
            phantom: PhantomData,
        }
    });

    UseState {
        value: BorrowedRcCell { inner: &hook.value },
        update_scope: hook.update_scope,
        phantom: PhantomData,
    }
}

pub struct UseState<'a, T, Storage: AsRef<RcCell<T>>> {
    pub(crate) update_scope: UpdateScope<'a>,
    pub(crate) value: Storage,
    phantom: PhantomData<T>,
}

impl<T, Storage: AsRef<RcCell<T>>> UseState<'_, T, Storage> {
    #[inline(always)]
    fn value(&self) -> &RcCell<T> {
        self.value.as_ref()
    }

    /// Set the state to a new value.
    pub fn set(&self, new: T) {
        *self.value().borrow_mut() = new;
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
    pub fn current(&self) -> Ref<'_, T> {
        self.value().borrow()
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
            let current = self.value().borrow();
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

    pub fn borrow(&self) -> Ref<'_, T> {
        self.current()
    }

    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        self.value().borrow_mut()
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
        apply(&mut self.value().borrow_mut());

        self.needs_update();
    }
}

impl<T, Storage: AsRef<RcCell<T>> + Clone> Clone for UseState<'_, T, Storage> {
    fn clone(&self) -> Self {
        Self {
            update_scope: self.update_scope,
            value: self.value.clone(),
            phantom: PhantomData,
        }
    }
}
impl<T, Storage: AsRef<RcCell<T>> + Copy> Copy for UseState<'_, T, Storage> {}

impl<T: Display, Storage: AsRef<RcCell<T>>> std::fmt::Display for UseState<'_, T, Storage> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &*self.value().borrow())
    }
}

impl<T: std::fmt::Binary, Storage: AsRef<RcCell<T>>> std::fmt::Binary for UseState<'_, T, Storage> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:b}", self.value().borrow().deref())
    }
}

impl<T: PartialEq, Storage: AsRef<RcCell<T>>> PartialEq<T> for UseState<'_, T, Storage> {
    fn eq(&self, other: &T) -> bool {
        *self.value().borrow().deref() == *other
    }
}

impl<T: PartialEq, Storage1: AsRef<RcCell<T>>, Storage2: AsRef<RcCell<T>>>
    PartialEq<UseState<'_, T, Storage2>> for UseState<'_, T, Storage1>
{
    fn eq(&self, other: &UseState<'_, T, Storage2>) -> bool {
        *self.value().borrow() == *other.value().borrow()
    }
}

impl<T: Debug, Storage: AsRef<RcCell<T>>> Debug for UseState<'_, T, Storage> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.value().borrow().deref())
    }
}

impl<T: Not + Copy, Storage: AsRef<RcCell<T>>> std::ops::Not for &UseState<'_, T, Storage> {
    type Output = <T as std::ops::Not>::Output;

    fn not(self) -> Self::Output {
        self.value().borrow().deref().not()
    }
}

impl<T: Not + Copy, Storage: AsRef<RcCell<T>>> std::ops::Not for UseState<'_, T, Storage> {
    type Output = <T as std::ops::Not>::Output;

    fn not(self) -> Self::Output {
        self.value().borrow().deref().not()
    }
}

impl<T: std::ops::Add + Copy, Storage: AsRef<RcCell<T>>> std::ops::Add<T>
    for &UseState<'_, T, Storage>
{
    type Output = <T as std::ops::Add>::Output;

    fn add(self, other: T) -> Self::Output {
        *self.value().borrow().deref() + other
    }
}
impl<T: std::ops::Sub + Copy, Storage: AsRef<RcCell<T>>> std::ops::Sub<T>
    for &UseState<'_, T, Storage>
{
    type Output = <T as std::ops::Sub>::Output;

    fn sub(self, other: T) -> Self::Output {
        *self.value().borrow().deref() - other
    }
}

impl<T: std::ops::Div + Copy, Storage: AsRef<RcCell<T>>> std::ops::Div<T>
    for &UseState<'_, T, Storage>
{
    type Output = <T as std::ops::Div>::Output;

    fn div(self, other: T) -> Self::Output {
        *self.value().borrow().deref() / other
    }
}

impl<T: std::ops::Mul + Copy, Storage: AsRef<RcCell<T>>> std::ops::Mul<T>
    for &UseState<'_, T, Storage>
{
    type Output = <T as std::ops::Mul>::Output;

    fn mul(self, other: T) -> Self::Output {
        *self.value().borrow().deref() * other
    }
}

impl<T: AddAssign, Storage: AsRef<RcCell<T>>> std::ops::AddAssign<T> for &UseState<'_, T, Storage> {
    fn add_assign(&mut self, rhs: T) {
        *self.value().borrow_mut() += rhs;
    }
}

impl<T: SubAssign, Storage: AsRef<RcCell<T>>> std::ops::SubAssign<T> for &UseState<'_, T, Storage> {
    fn sub_assign(&mut self, rhs: T) {
        *self.value().borrow_mut() -= rhs;
    }
}

impl<T: MulAssign, Storage: AsRef<RcCell<T>>> std::ops::MulAssign<T> for &UseState<'_, T, Storage> {
    fn mul_assign(&mut self, rhs: T) {
        *self.value().borrow_mut() *= rhs;
    }
}

impl<T: DivAssign, Storage: AsRef<RcCell<T>>> std::ops::DivAssign<T> for &UseState<'_, T, Storage> {
    fn div_assign(&mut self, rhs: T) {
        *self.value().borrow_mut() /= rhs;
    }
}

impl<T: AddAssign, Storage: AsRef<RcCell<T>>> std::ops::AddAssign<T> for UseState<'_, T, Storage> {
    fn add_assign(&mut self, rhs: T) {
        *self.value().borrow_mut() += rhs;
    }
}

impl<T: SubAssign, Storage: AsRef<RcCell<T>>> std::ops::SubAssign<T> for UseState<'_, T, Storage> {
    fn sub_assign(&mut self, rhs: T) {
        *self.value().borrow_mut() -= rhs;
    }
}

impl<T: MulAssign, Storage: AsRef<RcCell<T>>> std::ops::MulAssign<T> for UseState<'_, T, Storage> {
    fn mul_assign(&mut self, rhs: T) {
        *self.value().borrow_mut() *= rhs;
    }
}

impl<T: DivAssign, Storage: AsRef<RcCell<T>>> std::ops::DivAssign<T> for UseState<'_, T, Storage> {
    fn div_assign(&mut self, rhs: T) {
        *self.value().borrow_mut() /= rhs;
    }
}

#[test]
fn api_makes_sense() {
    #[allow(unused)]
    fn callback_like<'a, T>(scope: Scope<'a, T>, _: impl FnOnce() + 'a) {}

    #[allow(unused)]
    fn app(cx: Scope) -> Element {
        let mut val = use_state(cx, || 0);

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
            val += 1;
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

pub struct BorrowedRcCell<'a, T> {
    inner: &'a RcCell<T>,
}

impl<'a, T> Clone for BorrowedRcCell<'a, T> {
    fn clone(&self) -> Self {
        BorrowedRcCell { inner: self.inner }
    }
}

impl<'a, T> Copy for BorrowedRcCell<'a, T> {}

impl<'a, T> BorrowedRcCell<'a, T> {
    fn new(inner: &'a RcCell<T>) -> Self {
        Self { inner: inner }
    }

    fn borrow(&self) -> Ref<'_, T> {
        self.inner.borrow()
    }

    fn borrow_mut(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }

    fn to_owned(&self) -> RcCell<T> {
        self.inner.clone()
    }
}

impl<T> AsRef<RcCell<T>> for BorrowedRcCell<'_, T> {
    fn as_ref(&self) -> &RcCell<T> {
        self.inner.as_ref()
    }
}

pub struct RcCell<T> {
    inner: Rc<RefCell<T>>,
}

impl<T> AsRef<RcCell<T>> for RcCell<T> {
    fn as_ref(&self) -> &RcCell<T> {
        &self
    }
}

impl<T> Clone for RcCell<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> RcCell<T> {
    fn new(x: T) -> Self {
        Self {
            inner: Rc::new(RefCell::new(x)),
        }
    }

    fn borrow(&self) -> Ref<'_, T> {
        self.inner.borrow()
    }

    fn borrow_mut(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}

#[test]
fn sizes() {
    dbg!(std::mem::size_of::<UseState<i32, BorrowedRcCell<'_, i32>>>());
    dbg!(std::mem::size_of::<UseState<i32, RcCell<i32>>>());
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
