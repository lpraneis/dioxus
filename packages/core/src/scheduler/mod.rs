use crate::ScopeId;
use slab::Slab;

mod suspense;
mod task;
mod wait;
mod waker;

pub use suspense::*;
pub use task::*;
pub use waker::RcWake;

/// The type of message that can be sent to the scheduler.
///
/// These messages control how the scheduler will process updates to the UI.
#[derive(Debug)]
pub(crate) enum SchedulerMsg {
    /// Immediate updates from Components that mark them as dirty
    Immediate(ScopeId),

    /// A task has woken and needs to be progressed
    TaskNotified(TaskId),

    /// A task has woken and needs to be progressed
    SuspenseNotified(SuspenseId),
}

use std::{cell::RefCell, rc::Rc};

pub(crate) struct Scheduler {
    pub sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,

    pub copy_sender: Sender<'static, SchedulerMsg>,

    /// Tasks created with cx.spawn
    pub tasks: RefCell<Slab<Rc<LocalTask>>>,

    /// Async components
    pub leaves: RefCell<Slab<Rc<SuspenseLeaf>>>,
}

impl Scheduler {
    pub fn new(
        sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,
        copy_sender: Sender<'static, SchedulerMsg>,
    ) -> Rc<Self> {
        Rc::new(Scheduler {
            sender,
            copy_sender,
            tasks: RefCell::new(Slab::new()),
            leaves: RefCell::new(Slab::new()),
        })
    }
}

use std::{collections::VecDeque, fmt::Debug, marker::PhantomData};

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
