use crate::{arena::ElementId, AttributeValue, ScopeId};
use std::{any::Any, marker::PhantomData};

#[derive(Debug)]
pub struct Mutations<'a, M: MutationStore<'a>> {
    pub subtree: usize,
    pub template_mutations: M,
    pub edits: M,
    phantom: PhantomData<&'a ()>,
}

impl<'a, M: MutationStore<'a>> Mutations<'a, M> {
    pub fn new(subtree: usize) -> Self {
        Self {
            subtree,
            edits: M::default(),
            template_mutations: M::default(),
            phantom: PhantomData,
        }
    }
}

impl<'a, M: MutationStore<'a>> std::ops::Deref for Mutations<'a, M> {
    type Target = M;

    fn deref(&self) -> &Self::Target {
        &self.edits
    }
}

impl<'a, M: MutationStore<'a>> std::ops::DerefMut for Mutations<'a, M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.edits
    }
}

pub trait MutationStoreBuilder {
    type MutationStore<'a>: MutationStore<'a>;

    fn create<'a>() -> Self::MutationStore<'a>;
}

struct VecMutation;

impl MutationStoreBuilder for VecMutation {
    type MutationStore<'a> = Vec<Mutation<'a>>;

    fn create<'a>() -> Self::MutationStore<'a> {
        Vec::new()
    }
}

/*
each subtree has its own numbering scheme
*/

#[derive(Debug, PartialEq)]
pub enum Mutation<'a> {
    AppendChildren {
        m: usize,
    },

    AssignId {
        path: &'static [u8],
        id: ElementId,
    },

    CreateElement {
        name: &'a str,
        namespace: Option<&'a str>,
        id: ElementId,
    },

    CreatePlaceholder {
        id: ElementId,
    },
    CreateStaticText {
        value: &'a str,
    },
    CreateTextNode {
        value: &'a str,
        id: ElementId,
    },
    HydrateText {
        path: &'static [u8],
        value: &'a str,
        id: ElementId,
    },
    LoadTemplate {
        name: &'static str,
        index: usize,
    },

    // Take the current element and replace it with the element with the given id.
    ReplaceWith {
        id: ElementId,
        m: usize,
    },

    ReplacePlaceholder {
        m: usize,
        path: &'static [u8],
    },

    SaveTemplate {
        name: &'static str,
        m: usize,
    },

    SetAttribute {
        name: &'a str,
        value: &'a str,
        id: ElementId,

        // value: &'bump str,
        /// The (optional) namespace of the attribute.
        /// For instance, "style" is in the "style" namespace.
        ns: Option<&'a str>,
    },

    SetBoolAttribute {
        name: &'a str,
        value: bool,
        id: ElementId,

        // value: &'bump str,
        /// The (optional) namespace of the attribute.
        /// For instance, "style" is in the "style" namespace.
        ns: Option<&'a str>,
    },

    SetInnerText {
        value: &'a str,
    },

    SetText {
        value: &'a str,
        id: ElementId,
    },

    /// Create a new Event Listener.
    NewEventListener {
        /// The name of the event to listen for.
        event_name: &'a str,

        /// The ID of the node to attach the listener to.
        scope: ScopeId,

        /// The ID of the node to attach the listener to.
        id: ElementId,
    },

    /// Remove an existing Event Listener.
    RemoveEventListener {
        /// The ID of the node to remove.
        id: ElementId,

        /// The name of the event to remove.
        event: &'a str,
    },
}

pub trait MutationStore<'a>: Default {
    fn transmute_as_this(other: (impl MutationStore<'a> + Any)) -> Self {
        let boxed: Box<dyn Any> = Box::new(other);
        if let Ok(this) = boxed.downcast::<Self>() {
            *this
        } else {
            panic!("transmute_as_this failed");
        }
    }
    fn append(&mut self, other: Self);
    fn len(&self) -> usize;
    fn take(&mut self) -> Self {
        std::mem::take(self)
    }
    fn set_attribute(
        &mut self,
        name: &'a str,
        namespace: Option<&'a str>,
        value: &'a str,
        id: ElementId,
    );
    fn set_bool_attribute(
        &mut self,
        name: &'a str,
        namespace: Option<&'a str>,
        value: bool,
        id: ElementId,
    );
    fn load_template(&mut self, name: &'static str, index: usize);
    fn save_template(&mut self, name: &'static str, m: usize);
    fn hydrate_text(&mut self, path: &'static [u8], value: &'a str, id: ElementId);
    fn set_text(&mut self, value: &'a str, id: ElementId);
    fn replace_placeholder(&mut self, m: usize, path: &'static [u8]);
    fn assign_id(&mut self, path: &'static [u8], id: ElementId);
    fn replace(&mut self, id: ElementId, m: usize);
    fn create_element(&mut self, name: &'a str, namespace: Option<&'a str>, id: ElementId);
    fn set_inner_text(&mut self, value: &'a str);
    fn create_text(&mut self, id: ElementId, value: &'a str);
    fn create_static_text(&mut self, value: &'a str);
    fn create_placeholder(&mut self, id: ElementId);
    fn new_event_listener(&mut self, event_name: &'a str, scope: ScopeId, id: ElementId);
    fn remove_event_listener(&mut self, id: ElementId, event: &'a str);
    fn append_children(&mut self, m: usize);
}

impl<'a> MutationStore<'a> for Vec<Mutation<'a>> {
    fn append(&mut self, other: Self) {
        self.extend(other);
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn set_attribute(
        &mut self,
        name: &'a str,
        namespace: Option<&'a str>,
        value: &'a str,
        id: ElementId,
    ) {
        self.push(Mutation::SetAttribute {
            name,
            ns: namespace,
            value,
            id,
        });
    }

    fn load_template(&mut self, name: &'static str, index: usize) {
        self.push(Mutation::LoadTemplate { name, index });
    }

    fn save_template(&mut self, name: &'static str, m: usize) {
        self.push(Mutation::SaveTemplate { name, m });
    }

    fn hydrate_text(&mut self, path: &'static [u8], value: &'a str, id: ElementId) {
        self.push(Mutation::HydrateText { path, value, id });
    }

    fn set_text(&mut self, value: &'a str, id: ElementId) {
        self.push(Mutation::SetText { value, id });
    }

    fn replace_placeholder(&mut self, m: usize, path: &'static [u8]) {
        self.push(Mutation::ReplacePlaceholder { m, path });
    }

    fn assign_id(&mut self, path: &'static [u8], id: ElementId) {
        self.push(Mutation::AssignId { path, id });
    }

    fn replace(&mut self, id: ElementId, m: usize) {
        self.push(Mutation::ReplaceWith { id, m });
    }

    fn create_element(&mut self, name: &'a str, namespace: Option<&'a str>, id: ElementId) {
        self.push(Mutation::CreateElement {
            name,
            namespace,
            id,
        });
    }

    fn set_inner_text(&mut self, value: &'a str) {
        self.push(Mutation::SetInnerText { value });
    }

    fn create_text(&mut self, id: ElementId, value: &'a str) {
        self.push(Mutation::SetText { value, id });
    }

    fn create_static_text(&mut self, value: &'a str) {
        self.push(Mutation::CreateStaticText { value });
    }

    fn create_placeholder(&mut self, id: ElementId) {
        self.push(Mutation::CreatePlaceholder { id });
    }

    fn set_bool_attribute(
        &mut self,
        name: &'a str,
        namespace: Option<&'a str>,
        value: bool,
        id: ElementId,
    ) {
        self.push(Mutation::SetBoolAttribute {
            name,
            ns: namespace,
            value,
            id,
        });
    }

    fn new_event_listener(&mut self, event_name: &'a str, scope: ScopeId, id: ElementId) {
        self.push(Mutation::NewEventListener {
            event_name,
            scope,
            id,
        });
    }

    fn remove_event_listener(&mut self, id: ElementId, event: &'a str) {
        self.push(Mutation::RemoveEventListener { id, event });
    }

    fn append_children(&mut self, m: usize) {
        self.push(Mutation::AppendChildren { m });
    }
}
