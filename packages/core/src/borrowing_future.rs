use std::future::Future;
use std::pin::Pin;

use crate::innerlude::ScopeMap;

/// A future that may contain values borrowed from a [`Scope`].
#[repr(transparent)]
pub struct BorrowedFuture {
    fut: Pin<Box<dyn Future<Output = ()>>>,
}

impl BorrowedFuture {
    /// Create a new [`BorrowedFuture`] from a [`Future`].
    pub fn new(fut: Pin<Box<dyn Future<Output = ()>>>) -> Self {
        Self { fut }
    }

    pub(crate) fn write<'a>(
        &'a mut self,
        _: &'a mut ScopeMap,
    ) -> &'a mut Pin<Box<dyn Future<Output = ()>>> {
        &mut self.fut
    }
}
