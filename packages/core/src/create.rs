use std::ops::Bound;

use crate::factory::RenderReturn;
use crate::innerlude::{
    MutationStore, MutationStoreBuilder, Mutations, SuspenseContext, SuspenseId,
};
use crate::mutations::Mutation::*;
use crate::mutations::{self, Mutation};
use crate::nodes::VNode;
use crate::nodes::{DynamicNode, TemplateNode};
use crate::virtual_dom::VirtualDom;
use crate::{AttributeValue, ScopeId, TemplateAttribute};

impl<B: MutationStoreBuilder> VirtualDom<B> {
    pub(crate) fn create_scope<'a>(
        &mut self,
        scope: ScopeId,
        mutations: &mut Mutations<'a, B::MutationStore<'a>>,
        template: &'a VNode<'a>,
    ) -> usize {
        self.scope_stack.push(scope);
        let out = self.create(mutations, template);
        self.scope_stack.pop();
        out
    }

    /// Create this template and write its mutations
    pub(crate) fn create<'a>(
        &mut self,
        mutations: &mut Mutations<'a, B::MutationStore<'a>>,
        template: &'a VNode<'a>,
    ) -> usize {
        // The best renderers will have templates prehydrated and registered
        // Just in case, let's create the template using instructions anyways
        if !self.templates.contains_key(&template.template.id) {
            for node in template.template.roots {
                let mutations = &mut mutations.template_mutations;
                self.create_static_node(mutations, template, node);
            }

            mutations
                .template_mutations
                .save_template(template.template.id, template.template.roots.len());

            self.templates
                .insert(template.template.id, template.template.clone());
        }

        // Walk the roots, creating nodes and assigning IDs
        // todo: adjust dynamic nodes to be in the order of roots and then leaves (ie BFS)
        let mut dynamic_attrs = template.template.attr_paths.iter().enumerate().peekable();
        let mut dynamic_nodes = template.template.node_paths.iter().enumerate().peekable();

        let cur_scope = self.scope_stack.last().copied().unwrap();

        let mut on_stack = 0;
        for (root_idx, root) in template.template.roots.iter().enumerate() {
            on_stack += match root {
                TemplateNode::Element { .. } | TemplateNode::Text(_) => {
                    mutations.load_template(template.template.id, root_idx);
                    1
                }

                TemplateNode::DynamicText(id) | TemplateNode::Dynamic(id) => {
                    match &template.dynamic_nodes[*id] {
                        DynamicNode::Fragment { .. } | DynamicNode::Component { .. } => self
                            .create_dynamic_node(
                                mutations,
                                template,
                                &template.dynamic_nodes[*id],
                                *id,
                            ),
                        DynamicNode::Text {
                            id: slot, value, ..
                        } => {
                            let id = self.next_element(template);
                            slot.set(id);
                            mutations.create_text(id, value);
                            1
                        }
                        DynamicNode::Placeholder(slot) => {
                            let id = self.next_element(template);
                            slot.set(id);
                            mutations.create_placeholder(id);
                            1
                        }
                    }
                }
            };

            // we're on top of a node that has a dynamic attribute for a descendant
            // Set that attribute now before the stack gets in a weird state
            while let Some((mut attr_id, path)) =
                dynamic_attrs.next_if(|(_, p)| p[0] == root_idx as u8)
            {
                let id = self.next_element(template);
                mutations.assign_id(&path[1..], id);

                loop {
                    let attribute = template.dynamic_attrs.get(attr_id).unwrap();
                    attribute.mounted_element.set(id);

                    match &attribute.value {
                        AttributeValue::Text(value) => {
                            mutations.set_attribute(attribute.name, attribute.namespace, *value, id)
                        }
                        AttributeValue::Bool(value) => mutations.set_bool_attribute(
                            attribute.name,
                            attribute.namespace,
                            *value,
                            id,
                        ),
                        AttributeValue::Listener(_) => {
                            mutations.new_event_listener(attribute.name, cur_scope, id)
                        }
                        AttributeValue::Float(_) => todo!(),
                        AttributeValue::Int(_) => todo!(),
                        AttributeValue::Any(_) => todo!(),
                        AttributeValue::None => todo!(),
                    }

                    // Only push the dynamic attributes forward if they match the current path (same element)
                    match dynamic_attrs.next_if(|(_, p)| *p == path) {
                        Some((next_attr_id, _)) => attr_id = next_attr_id,
                        None => break,
                    }
                }
            }

            // We're on top of a node that has a dynamic child for a descendant
            // Skip any node that's a root
            let mut start = None;
            let mut end = None;

            while let Some((idx, p)) = dynamic_nodes.next_if(|(_, p)| p[0] == root_idx as u8) {
                if p.len() == 1 {
                    continue;
                }

                if start.is_none() {
                    start = Some(idx);
                }

                end = Some(idx);
            }

            if let (Some(start), Some(end)) = (start, end) {
                for idx in start..=end {
                    let node = &template.dynamic_nodes[idx];
                    let m = self.create_dynamic_node(mutations, template, node, idx);
                    if m > 0 {
                        mutations.replace_placeholder(m, &template.template.node_paths[idx][1..]);
                    }
                }
            }
        }

        on_stack
    }

    pub(crate) fn create_static_node<'a>(
        &mut self,
        mutations: &mut B::MutationStore<'a>,
        template: &'a VNode<'a>,
        node: &'a TemplateNode<'static>,
    ) {
        match *node {
            // Todo: create the children's template
            TemplateNode::Dynamic(_) => {
                let id = self.next_element(template);
                mutations.create_placeholder(id);
            }
            TemplateNode::Text(value) => {
                mutations.create_static_text(value);
            }
            TemplateNode::DynamicText { .. } => {
                mutations.create_static_text("placeholder");
            }

            TemplateNode::Element {
                attrs,
                children,
                namespace,
                tag,
                inner_opt,
            } => {
                let id = self.next_element(template);

                mutations.create_element(tag, namespace, id);

                for attr in attrs {
                    match attr {
                        TemplateAttribute::Static {
                            name,
                            value,
                            namespace,
                            ..
                        } => {
                            mutations.set_attribute(name, *namespace, value, id);
                        }
                        _ => (),
                    }
                }

                if children.is_empty() && inner_opt {
                    return;
                }

                children
                    .into_iter()
                    .for_each(|child| self.create_static_node(mutations, template, child));

                mutations.append_children(children.len())
            }
        }
    }

    pub(crate) fn create_dynamic_node<'a>(
        &mut self,
        mutations: &mut Mutations<'a, B::MutationStore<'a>>,
        template: &'a VNode<'a>,
        node: &'a DynamicNode<'a>,
        idx: usize,
    ) -> usize {
        match &node {
            DynamicNode::Text { id, value, .. } => {
                let new_id = self.next_element(template);
                id.set(new_id);
                mutations.hydrate_text(&template.template.node_paths[idx][1..], value, new_id);
                0
            }

            DynamicNode::Component {
                props,
                placeholder,
                scope: scope_slot,
                ..
            } => {
                let scope = self
                    .new_scope(unsafe { std::mem::transmute(props.get()) })
                    .id;

                scope_slot.set(Some(scope));

                let return_nodes = unsafe { self.run_scope(scope).extend_lifetime_ref() };

                match return_nodes {
                    RenderReturn::Sync(None) => {
                        todo!()
                    }

                    RenderReturn::Async(_) => {
                        let new_id = self.next_element(template);
                        placeholder.set(Some(new_id));
                        self.scopes[scope.0].placeholder.set(Some(new_id));

                        mutations.assign_id(&template.template.node_paths[idx][1..], new_id);

                        let boudary = self.scopes[scope.0]
                            .consume_context::<SuspenseContext<B::MutationStore<'static>>>()
                            .unwrap();

                        if boudary.placeholder.get().is_none() {
                            boudary.placeholder.set(Some(new_id));
                        }
                        boudary
                            .waiting_on
                            .borrow_mut()
                            .extend(self.collected_leaves.drain(..));

                        0
                    }

                    RenderReturn::Sync(Some(template)) => {
                        if !self.collected_leaves.is_empty() {
                            if let Some(boundary) = self.scopes[scope.0]
                                .has_context::<SuspenseContext<B::MutationStore<'static>>>()
                            {
                                // Since this is a boundary, use it as a placeholder
                                let new_id = self.next_element(&template);
                                placeholder.set(Some(new_id));
                                self.scopes[scope.0].placeholder.set(Some(new_id));
                                mutations
                                    .assign_id(&template.template.node_paths[idx][1..], new_id);

                                // Now connect everything to the boundary
                                let boundary_mut = boundary;
                                let split_off = mutations.take();
                                let split_off: B::MutationStore<'static> =
                                    unsafe { std::mem::transmute(split_off) };

                                self.scope_stack.push(scope);
                                let mut created = self.create(mutations, template);
                                self.scope_stack.pop();

                                if boundary_mut.placeholder.get().is_none() {
                                    boundary_mut.placeholder.set(Some(new_id));
                                }

                                // In the generated edits, we want to pick off from where we left off.
                                boundary_mut.mutations.borrow_mut().edits.append(split_off);

                                boundary_mut
                                    .waiting_on
                                    .borrow_mut()
                                    .extend(self.collected_leaves.drain(..));

                                created = 0;
                                return created;
                            }
                        }
                        self.scope_stack.push(scope);
                        let mut created = self.create(mutations, template);
                        self.scope_stack.pop();

                        // handle any waiting on futures accumulated by async calls down the tree
                        // if this is a boundary, we split off the tree
                        created
                    }
                }
            }

            DynamicNode::Fragment { nodes, .. } => {
                //
                nodes
                    .iter()
                    .fold(0, |acc, child| acc + self.create(mutations, child))
            }

            DynamicNode::Placeholder(slot) => {
                let id = self.next_element(template);
                slot.set(id);
                mutations.assign_id(&template.template.node_paths[idx][1..], id);

                0
            }
        }
    }
}
