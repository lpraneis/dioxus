use crate::{virtual_dom::VirtualDom, ScopeId, ScopeState};
use std::{
    any::Any,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A weak reference to a [`Hook`]
#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(transparent)]
pub struct WeakHook<T> {
    pub(crate) ptr: *mut T,
}

impl<'b, T: 'static> WeakHook<T> {
    #[doc(hidden)]
    pub unsafe fn upgrade_raw(&self) -> Hook<'b, T> {
        Hook {
            data: unsafe { &mut *self.ptr },
        }
    }
}

/// A handle to some data created in a [`Scope`]
#[derive(Debug, PartialEq)]
pub struct Hook<'b, T> {
    pub(crate) data: &'b mut T,
}

impl<'b, T: 'static> Deref for Hook<'b, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'b, T: 'static> DerefMut for Hook<'b, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<'b, T: 'static> Hook<'b, T> {
    pub(crate) fn dwgrd(&mut self) -> WeakHook<T> {
        WeakHook {
            ptr: unsafe { self.data as *mut T },
        }
    }
}
