#![warn(clippy::pedantic)]

use dioxus_core::prelude::*;
use std::{
    cell::{Ref, RefCell, RefMut},
    fmt::{Debug, Display},
    ops::{Add, Div, Mul, Not, Sub},
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
///     let count = use_state(cx, || 0);
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
pub fn use_state<T: 'static>(
    cx: &ScopeState,
    initial_state_fn: impl FnOnce() -> T,
) -> &UseState<T> {
    cx.use_hook(move || {
        let update_callback = cx.schedule_update();
        let slot = Rc::new(RefCell::new(initial_state_fn()));
        let setter = Rc::new({
            to_owned![update_callback, slot];
            move |new| {
                {
                    let mut slot = slot.borrow_mut();
                    *slot = new;
                }
                update_callback();
            }
        });

        UseState {
            update_callback,
            setter,
            slot,
        }
    })
}

pub struct UseState<T: 'static> {
    pub(crate) update_callback: Arc<dyn Fn()>,
    pub(crate) setter: Rc<dyn Fn(T)>,
    pub(crate) slot: Rc<RefCell<T>>,
}

impl<T: 'static> UseState<T> {
    /// Set the state to a new value.
    pub fn set(&self, new: T) {
        (self.setter)(new);
    }

    /// Get the `setter` function directly without the `UseState` wrapper.
    ///
    /// This is useful for passing the setter function to other components.
    ///
    /// However, for most cases, calling `to_owned` on the state is the
    /// preferred way to get "another" state handle.
    ///
    ///
    /// # Examples
    /// A component might require an `Rc<dyn Fn(T)>` as an input to set a value.
    ///
    /// ```rust, ignore
    /// fn component(cx: Scope) -> Element {
    ///     let value = use_state(cx, || 0);
    ///
    ///     rsx!{
    ///         Component {
    ///             handler: value.setter()
    ///         }
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn setter(&self) -> Rc<dyn Fn(T)> {
        self.setter.clone()
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
    ///     let value = use_state(cx, || 0);
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
        let new_val = f(&*self.read());
        (self.setter)(new_val);
    }

    /// Get the value of the state when this handle was created.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// # use dioxus_core::prelude::*;
    /// # use dioxus_hooks::*;
    /// fn component(cx: Scope) -> Element {
    ///     let value = use_state(cx, || 0);
    ///
    ///     let current = value.get();
    ///     assert_eq!(current, &0);
    ///
    ///     # todo!()
    /// }
    /// ```
    #[must_use]
    pub fn read(&self) -> Ref<'_, T> {
        self.slot.borrow()
    }

    /// Mutably unlock the value in the ```RefCell```. This will mark the component as "dirty"
    ///
    /// Uses to `borrow_mut` should be as short as possible.
    ///
    /// Be very careful when working with this method. If you can, consider using
    /// the `with` and `with_mut` methods instead, choosing to render Elements
    /// during the read and write calls.
    #[must_use]
    pub fn write(&self) -> RefMut<'_, T> {
        self.needs_update();
        self.slot.borrow_mut()
    }

    /// Take a reference to the inner value termporarily and produce a new value
    ///
    /// Note: You can always "reborrow" the value through the ```RefCell```.
    /// This method just does it for you automatically.
    ///
    /// ```rust, ignore
    /// let val = use_state(|| HashMap::<u32, String>::new());
    ///
    ///
    /// // use reborrowing
    /// let inner = &*val.read();
    ///
    /// // or, be safer and use `with`
    /// val.with(|i| println!("{:?}", i));
    /// ```
    pub fn with<O>(&self, immutable_callback: impl FnOnce(&T) -> O) -> O {
        immutable_callback(&*self.read())
    }

    /// Take a reference to the inner value termporarily and produce a new value,
    /// modifying the original in place.
    ///
    /// Note: You can always "reborrow" the value through the ```RefCell```.
    /// This method just does it for you automatically.
    ///
    /// ```rust, ignore
    /// let val = use_state(|| HashMap::<u32, String>::new());
    ///
    ///
    /// // use reborrowing
    /// let inner = &mut *val.write();
    ///
    /// // or, be safer and use `with`
    /// val.with_mut(|i| i.insert(1, "hi"));
    /// ```
    pub fn with_mut<O>(&self, mutable_callback: impl FnOnce(&mut T) -> O) -> O {
        mutable_callback(&mut *self.write())
    }

    /// Mark the component that created this [`UseState`] as dirty, forcing it to re-render.
    ///
    /// ```rust, ignore
    /// fn component(cx: Scope) -> Element {
    ///     let count = use_state(cx, || 0);
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
        (self.update_callback)();
    }
}

impl<T: 'static> Clone for UseState<T> {
    fn clone(&self) -> Self {
        UseState {
            update_callback: self.update_callback.clone(),
            setter: self.setter.clone(),
            slot: self.slot.clone(),
        }
    }
}

impl<T: 'static + Display> std::fmt::Display for UseState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.read())
    }
}

impl<T: std::fmt::Binary> std::fmt::Binary for UseState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:b}", *self.read())
    }
}

impl<T: PartialEq> PartialEq<T> for UseState<T> {
    fn eq(&self, other: &T) -> bool {
        &*self.read() == other
    }
}

// todo: this but for more interesting conrete types
impl PartialEq<bool> for &UseState<bool> {
    fn eq(&self, other: &bool) -> bool {
        &*self.read() == other
    }
}

impl<T: PartialEq> PartialEq<UseState<T>> for UseState<T> {
    fn eq(&self, other: &UseState<T>) -> bool {
        Rc::ptr_eq(&self.slot, &other.slot) || { *self.read() == *other.read() }
    }
}

impl<T: Debug> Debug for UseState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.read())
    }
}

impl<T: Not + Copy> std::ops::Not for &UseState<T> {
    type Output = <T as std::ops::Not>::Output;

    fn not(self) -> Self::Output {
        self.read().not()
    }
}

impl<T: Not + Copy> std::ops::Not for UseState<T> {
    type Output = <T as std::ops::Not>::Output;

    fn not(self) -> Self::Output {
        self.read().not()
    }
}

impl<T: std::ops::Add + Copy> std::ops::Add<T> for &UseState<T> {
    type Output = <T as std::ops::Add>::Output;

    fn add(self, other: T) -> Self::Output {
        *self.read() + other
    }
}
impl<T: std::ops::Sub + Copy> std::ops::Sub<T> for &UseState<T> {
    type Output = <T as std::ops::Sub>::Output;

    fn sub(self, other: T) -> Self::Output {
        *self.read() - other
    }
}

impl<T: std::ops::Div + Copy> std::ops::Div<T> for &UseState<T> {
    type Output = <T as std::ops::Div>::Output;

    fn div(self, other: T) -> Self::Output {
        *self.read() / other
    }
}

impl<T: std::ops::Mul + Copy> std::ops::Mul<T> for &UseState<T> {
    type Output = <T as std::ops::Mul>::Output;

    fn mul(self, other: T) -> Self::Output {
        *self.read() * other
    }
}

impl<T: Add<Output = T> + Copy> std::ops::AddAssign<T> for &UseState<T> {
    fn add_assign(&mut self, rhs: T) {
        self.set((*self.read()) + rhs);
    }
}

impl<T: Sub<Output = T> + Copy> std::ops::SubAssign<T> for &UseState<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.set((*self.read()) - rhs);
    }
}

impl<T: Mul<Output = T> + Copy> std::ops::MulAssign<T> for &UseState<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.set((*self.read()) * rhs);
    }
}

impl<T: Div<Output = T> + Copy> std::ops::DivAssign<T> for &UseState<T> {
    fn div_assign(&mut self, rhs: T) {
        self.set((*self.read()) / rhs);
    }
}

impl<T: Add<Output = T> + Copy> std::ops::AddAssign<T> for UseState<T> {
    fn add_assign(&mut self, rhs: T) {
        self.set((*self.read()) + rhs);
    }
}

impl<T: Sub<Output = T> + Copy> std::ops::SubAssign<T> for UseState<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.set((*self.read()) - rhs);
    }
}

impl<T: Mul<Output = T> + Copy> std::ops::MulAssign<T> for UseState<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.set((*self.read()) * rhs);
    }
}

impl<T: Div<Output = T> + Copy> std::ops::DivAssign<T> for UseState<T> {
    fn div_assign(&mut self, rhs: T) {
        self.set((*self.read()) / rhs);
    }
}

#[test]
fn api_makes_sense() {
    #[allow(unused)]
    fn app(cx: Scope) -> Element {
        let val = use_state(cx, || 0);

        val.set(0);
        val.modify(|v| v + 1);
        let real_current = val.read();

        match *val.read() {
            10 => {
                val.set(20);
                val.modify(|v| v + 1);
            }
            20 => {}
            _ => {
                println!("{real_current}");
            }
        }

        cx.spawn({
            to_owned![val];
            async move {
                val.modify(|f| f + 1);
            }
        });

        // cx.render(LazyNodes::new(|f| f.static_text("asd")))

        todo!()
    }
}
