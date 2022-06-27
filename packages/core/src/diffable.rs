use fxhash::{FxHashMap, FxHashSet};

use crate::{
    innerlude::ScopeArena, ElementId, Mutations, VComponent, VElement, VFragment, VNode,
    VPlaceholder, VText,
};

trait Diffable<'a> {
    type Regestry;

    fn create(&self, registry: &mut Self::Regestry);

    fn destroy(&self, registry: &mut Self::Regestry);

    fn diff(&self, old: &Self, registry: &mut Self::Regestry);
}

struct VDomRegestry<'a> {
    force_diff: bool,
    nodes_to_place: usize,
    mutations: &'a mut Mutations<'a>,
    scopes: &'a ScopeArena,
}

impl<'a> VDomRegestry<'a> {
    fn take_created(&mut self) -> usize {
        std::mem::take(&mut self.nodes_to_place)
    }
}

impl<'a> Diffable<'a> for VText<'a> {
    type Regestry = VDomRegestry<'a>;

    fn create(&self, registry: &mut Self::Regestry) {
        let id = self.id.get().unwrap();
        registry.mutations.create_text_node(self.text, id);
    }

    fn destroy(&self, registry: &mut Self::Regestry) {
        // this check exists because our null node will be removed but does not have an ID
        if let Some(id) = self.id.get() {
            registry.mutations.remove(id.as_u64());
        }
    }

    fn diff(&self, old: &Self, registry: &mut Self::Regestry) {
        if std::ptr::eq(self, old) {
            return;
        }

        if self.text != old.text {
            let id = self.id.get().unwrap().as_u64();
            registry.mutations.set_text(old.text, id);
        }
    }
}

impl<'a> Diffable<'a> for VPlaceholder {
    type Regestry = VDomRegestry<'a>;

    fn create(&self, registry: &mut Self::Regestry) {
        let id = self.id.get().unwrap();
        registry.mutations.create_placeholder(id);
    }

    fn destroy(&self, registry: &mut Self::Regestry) {
        let id = self.id.get().unwrap();
        registry.mutations.remove(id.as_u64());
    }

    fn diff(&self, _: &Self, _: &mut Self::Regestry) {}
}

impl<'a> Diffable<'a> for VElement<'a> {
    type Regestry = VDomRegestry<'a>;

    fn create(&self, registry: &mut Self::Regestry) {
        todo!()
    }

    fn destroy(&self, registry: &mut Self::Regestry) {
        todo!()
    }

    fn diff(&self, old: &Self, registry: &mut Self::Regestry) {
        if std::ptr::eq(self, old) {
            return;
        }

        let root = self.id.get().unwrap();

        // If the element type is completely different, the element needs to be re-rendered completely
        // This is an optimization React makes due to how users structure their code
        //
        // This case is rather rare (typically only in non-keyed lists)
        if self.tag != old.tag || self.namespace != old.namespace {
            // self.replace_node(old_node, new_node);
            return;
        }

        // todo: attributes currently rely on the element on top of the stack, but in theory, we only need the id of the
        // element to modify its attributes.
        // it would result in fewer instructions if we just set the id directly.
        // it would also clean up this code some, but that's not very important anyways

        // Diff Attributes
        //
        // It's extraordinarily rare to have the number/order of attributes change
        // In these cases, we just completely erase the old set and make a new set
        //
        // TODO: take a more efficient path than this
        if old.attributes.len() == self.attributes.len() {
            for (old_attr, new_attr) in old.attributes.iter().zip(self.attributes.iter()) {
                if old_attr.value != new_attr.value || new_attr.is_volatile {
                    registry.mutations.set_attribute(new_attr, root.as_u64());
                }
            }
        } else {
            for attribute in old.attributes {
                registry
                    .mutations
                    .remove_attribute(attribute, root.as_u64());
            }
            for attribute in self.attributes {
                registry.mutations.set_attribute(attribute, root.as_u64());
            }
        }

        // Diff listeners
        //
        // It's extraordinarily rare to have the number/order of listeners change
        // In the cases where the listeners change, we completely wipe the data attributes and add new ones
        //
        // We also need to make sure that all listeners are properly attached to the parent scope (fix_listener)
        //
        // TODO: take a more efficient path than this

        if old.listeners.len() == self.listeners.len() {
            for (old_l, new_l) in old.listeners.iter().zip(self.listeners.iter()) {
                if old_l.event != new_l.event {
                    registry
                        .mutations
                        .remove_event_listener(old_l.event, root.as_u64());
                    registry.mutations.new_event_listener(new_l);
                }
                new_l.mounted_node.set(old_l.mounted_node.get());
            }
        } else {
            for listener in old.listeners {
                registry
                    .mutations
                    .remove_event_listener(listener.event, root.as_u64());
            }
            for listener in self.listeners {
                listener.mounted_node.set(Some(root));
                registry.mutations.new_event_listener(listener);
            }
        }

        // match (old.children.len(), self.children.len()) {
        //     (0, 0) => {}
        //     (0, _) => {
        //         registry.push_root(root);
        //         let created = self.create_children(self.children);
        //         registry.append_children(created as u32);
        //         registry.pop_root();
        //     }
        //     (_, _) => self.diff_children(old.children, self.children),
        // };
    }
}

impl<'a> Diffable<'a> for VComponent<'a> {
    type Regestry = VDomRegestry<'a>;

    fn create(&self, registry: &mut Self::Regestry) {
        todo!()
    }

    fn destroy(&self, registry: &mut Self::Regestry) {
        todo!()
    }

    fn diff(&self, old: &Self, registry: &mut Self::Regestry) {
        let scope_addr = old
            .scope
            .get()
            .expect("existing component nodes should have a scope");

        if std::ptr::eq(self, old) {
            return;
        }

        // Make sure we're dealing with the same component (by function pointer)
        if old.user_fc == self.user_fc {
            // self.enter_scope(scope_addr);
            {
                // Make sure the new component vnode is referencing the right scope id
                // new.scope.set(Some(scope_addr));

                // make sure the component's caller function is up to date
                let scope = registry
                    .scopes
                    .get_scope(scope_addr)
                    .unwrap_or_else(|| panic!("could not find {:?}", scope_addr));

                // take the new props out regardless
                // when memoizing, push to the existing scope if memoization happens
                let new_props = self
                    .props
                    .borrow_mut()
                    .take()
                    .expect("new component props should exist");

                let should_diff = {
                    if old.can_memoize {
                        // safety: we trust the implementation of "memoize"
                        let props_are_the_same = unsafe {
                            let new_ref = new_props.as_ref();
                            scope.props.borrow().as_ref().unwrap().memoize(new_ref)
                        };
                        !props_are_the_same || registry.force_diff
                    } else {
                        true
                    }
                };

                if should_diff {
                    let _old_props = scope
                        .props
                        .replace(unsafe { std::mem::transmute(Some(new_props)) });

                    // this should auto drop the previous props
                    registry.scopes.run_scope(scope_addr);
                    registry.mutations.mark_dirty_scope(scope_addr);

                    registry
                        .scopes
                        .fin_head(scope_addr)
                        .diff(registry.scopes.wip_head(scope_addr), registry);
                } else {
                    // memoization has taken place
                    drop(new_props);
                };
            }
            // self.leave_scope();
        } else {
            old.destroy(registry);
            self.create(registry);
            // self.replace_node(old_node, new_node);
        }
    }
}

impl<'a> Diffable<'a> for VFragment<'a> {
    type Regestry = VDomRegestry<'a>;

    fn create(&self, registry: &mut Self::Regestry) {
        todo!()
    }

    fn destroy(&self, registry: &mut Self::Regestry) {
        todo!()
    }

    fn diff(&self, old: &Self, registry: &mut Self::Regestry) {
        if std::ptr::eq(self, old) {
            return;
        }

        // This is the case where options or direct vnodes might be used.
        // In this case, it's faster to just skip ahead to their diff
        if old.children.len() == 1 && self.children.len() == 1 {
            self.children[0].diff(&old.children[0], registry);
            return;
        }

        debug_assert!(!old.children.is_empty());
        debug_assert!(!self.children.is_empty());

        self.diff_children(old.children, self.children, registry);
    }
}

impl<'a> Diffable<'a> for VNode<'a> {
    type Regestry = VDomRegestry<'a>;

    fn diff(&self, old: &Self, registry: &mut Self::Regestry) {
        use VNode::*;
        match (self, old) {
            (Text(new), Text(old)) => {
                new.diff(old, registry);
            }

            (Placeholder(new), Placeholder(old)) => {
                new.diff(old, registry);
            }

            (Element(new), Element(old)) => {
                new.diff(old, registry);
            }

            (Component(new), Component(old)) => {
                new.diff(old, registry);
            }

            (Fragment(new), Fragment(old)) => {
                new.diff(old, registry);
            }

            (
                Component(_) | Fragment(_) | Text(_) | Element(_) | Placeholder(_),
                Component(_) | Fragment(_) | Text(_) | Element(_) | Placeholder(_),
            ) => {
                old.destroy(registry);
                self.create(registry);
            }
        }
    }

    fn create(&self, registry: &mut Self::Regestry) {
        match self {
            VNode::Text(new) => new.create(registry),
            VNode::Element(new) => new.create(registry),
            VNode::Fragment(new) => new.create(registry),
            VNode::Component(new) => new.create(registry),
            VNode::Placeholder(new) => new.create(registry),
        }
    }

    fn destroy(&self, registry: &mut Self::Regestry) {
        match self {
            VNode::Text(old) => old.destroy(registry),
            VNode::Element(old) => old.destroy(registry),
            VNode::Fragment(old) => old.destroy(registry),
            VNode::Component(old) => old.destroy(registry),
            VNode::Placeholder(old) => old.destroy(registry),
        }
    }
}

impl<'b> Diffable<'b> for &'b [VNode<'b>] {
    type Regestry = VDomRegestry<'b>;

    fn create(&self, registry: &mut Self::Regestry) {
        for node in self {
            node.create(node);
        }
    }

    fn destroy(&self, registry: &mut Self::Regestry) {
        for node in self {
            node.distroy(registry);
        }
    }

    // Diff the given set of old and new children.
    //
    // The parent must be on top of the change list stack when this function is
    // entered:
    //
    //     [... parent]
    //
    // the change list stack is in the same state when this function returns.
    //
    // If old no anchors are provided, then it's assumed that we can freely append to the parent.
    //
    // Remember, non-empty lists does not mean that there are real elements, just that there are virtual elements.
    //
    // Fragment nodes cannot generate empty children lists, so we can assume that when a list is empty, it belongs only
    // to an element, and appending makes sense.
    fn diff(&self, old: &Self, registry: &mut Self::Regestry) {
        if std::ptr::eq(old, self) {
            return;
        }

        // Remember, fragments can never be empty (they always have a single child)
        match (old, self) {
            ([], []) => {}
            ([], _) => create_and_append_children(self, registry),
            (_, []) => old.destroy(registry),
            _ => {
                let new_is_keyed = self[0].key().is_some();
                let old_is_keyed = old[0].key().is_some();

                debug_assert!(
                    self.iter().all(|n| n.key().is_some() == new_is_keyed),
                    "all siblings must be keyed or all siblings must be non-keyed"
                );
                debug_assert!(
                    old.iter().all(|o| o.key().is_some() == old_is_keyed),
                    "all siblings must be keyed or all siblings must be non-keyed"
                );

                if new_is_keyed && old_is_keyed {
                    diff_keyed_children(old, self, registry);
                } else {
                    diff_non_keyed_children(old, self, registry);
                }
            }
        }
    }
}

// Diff children that are not keyed.
//
// The parent must be on the top of the change list stack when entering this
// function:
//
//     [... parent]
//
// the change list stack is in the same state when this function returns.
fn diff_non_keyed_children<'b>(
    old: &'b [VNode<'b>],
    new: &'b [VNode<'b>],
    registry: &mut VDomRegestry<'b>,
) {
    use std::cmp::Ordering;

    // Handled these cases in `diff_children` before calling this function.
    debug_assert!(!new.is_empty());
    debug_assert!(!old.is_empty());

    match old.len().cmp(&new.len()) {
        Ordering::Greater => old[new.len()..].for_each(|n| n.destroy(registry)),
        Ordering::Less => create_and_insert_after(&new[old.len()..], old.last().unwrap(), registry),
        Ordering::Equal => {}
    }

    for (new, old) in new.iter().zip(old.iter()) {
        new.diff(old, registry);
    }
}

// Diffing "keyed" children.
//
// With keyed children, we care about whether we delete, move, or create nodes
// versus mutate existing nodes in place. Presumably there is some sort of CSS
// transition animation that makes the virtual DOM diffing algorithm
// observable. By specifying keys for nodes, we know which virtual DOM nodes
// must reuse (or not reuse) the same physical DOM nodes.
//
// This is loosely based on Inferno's keyed patching implementation. However, we
// have to modify the algorithm since we are compiling the diff down into change
// list instructions that will be executed later, rather than applying the
// changes to the DOM directly as we compare virtual DOMs.
//
// https://github.com/infernojs/inferno/blob/36fd96/packages/inferno/src/DOM/patching.ts#L530-L739
//
// The stack is empty upon entry.
fn diff_keyed_children<'b>(
    old: &'b [VNode<'b>],
    new: &'b [VNode<'b>],
    registry: &mut VDomRegestry<'b>,
) {
    if cfg!(debug_assertions) {
        let mut keys = fxhash::FxHashSet::default();
        let mut assert_unique_keys = |children: &'b [VNode<'b>]| {
            keys.clear();
            for child in children {
                let key = child.key();
                debug_assert!(
                    key.is_some(),
                    "if any sibling is keyed, all siblings must be keyed"
                );
                keys.insert(key);
            }
            debug_assert_eq!(
                children.len(),
                keys.len(),
                "keyed siblings must each have a unique key"
            );
        };
        assert_unique_keys(old);
        assert_unique_keys(new);
    }

    // First up, we diff all the nodes with the same key at the beginning of the
    // children.
    //
    // `shared_prefix_count` is the count of how many nodes at the start of
    // `new` and `old` share the same keys.
    let (left_offset, right_offset) = match diff_keyed_ends(old, new, registry) {
        Some(count) => count,
        None => return,
    };

    // Ok, we now hopefully have a smaller range of children in the middle
    // within which to re-order nodes with the same keys, remove old nodes with
    // now-unused keys, and create new nodes with fresh keys.

    let old_middle = &old[left_offset..(old.len() - right_offset)];
    let new_middle = &new[left_offset..(new.len() - right_offset)];

    debug_assert!(
        !((old_middle.len() == new_middle.len()) && old_middle.is_empty()),
        "keyed children must have the same number of children"
    );

    if new_middle.is_empty() {
        // remove the old elements
        old_middle.destroy(registry);
    } else if old_middle.is_empty() {
        // there were no old elements, so just create the new elements
        // we need to find the right "foothold" though - we shouldn't use the "append" at all
        if left_offset == 0 {
            // insert at the beginning of the old list
            let foothold = &old[old.len() - right_offset];
            create_and_insert_before(new_middle, foothold, registry);
        } else if right_offset == 0 {
            // insert at the end  the old list
            let foothold = old.last().unwrap();
            create_and_insert_after(new_middle, foothold, registry);
        } else {
            // inserting in the middle
            let foothold = &old[left_offset - 1];
            create_and_insert_after(new_middle, foothold, registry);
        }
    } else {
        diff_keyed_middle(old_middle, new_middle, registry);
    }
}

/// Diff both ends of the children that share keys.
///
/// Returns a left offset and right offset of that indicates a smaller section to pass onto the middle diffing.
///
/// If there is no offset, then this function returns None and the diffing is complete.
fn diff_keyed_ends<'b>(
    old: &'b [VNode<'b>],
    new: &'b [VNode<'b>],
    registry: &mut VDomRegestry<'b>,
) -> Option<(usize, usize)> {
    let mut left_offset = 0;

    for (old, new) in old.iter().zip(new.iter()) {
        // abort early if we finally run into nodes with different keys
        if old.key() != new.key() {
            break;
        }
        new.diff(old, registry);
        left_offset += 1;
    }

    // If that was all of the old children, then create and append the remaining
    // new children and we're finished.
    if left_offset == old.len() {
        create_and_insert_after(&new[left_offset..], old.last().unwrap(), registry);
        return None;
    }

    // And if that was all of the new children, then remove all of the remaining
    // old children and we're finished.
    if left_offset == new.len() {
        old[left_offset..].destroy(registry);
        return None;
    }

    // if the shared prefix is less than either length, then we need to walk backwards
    let mut right_offset = 0;
    for (old, new) in old.iter().rev().zip(new.iter().rev()) {
        // abort early if we finally run into nodes with different keys
        if old.key() != new.key() {
            break;
        }
        new.diff(old, registry);
        right_offset += 1;
    }

    Some((left_offset, right_offset))
}

// The most-general, expensive code path for keyed children diffing.
//
// We find the longest subsequence within `old` of children that are relatively
// ordered the same way in `new` (via finding a longest-increasing-subsequence
// of the old child's index within `new`). The children that are elements of
// this subsequence will remain in place, minimizing the number of DOM moves we
// will have to do.
//
// Upon entry to this function, the change list stack must be empty.
//
// This function will load the appropriate nodes onto the stack and do diffing in place.
//
// Upon exit from this function, it will be restored to that same self.
#[allow(clippy::too_many_lines)]
fn diff_keyed_middle<'b>(
    old: &'b [VNode<'b>],
    new: &'b [VNode<'b>],
    registry: &mut VDomRegestry<'b>,
) {
    /*
    1. Map the old keys into a numerical ordering based on indices.
    2. Create a map of old key to its index
    3. Map each new key to the old key, carrying over the old index.
        - IE if we have ABCD becomes BACD, our sequence would be 1,0,2,3
        - if we have ABCD to ABDE, our sequence would be 0,1,3,MAX because E doesn't exist

    now, we should have a list of integers that indicates where in the old list the new items map to.

    4. Compute the LIS of this list
        - this indicates the longest list of new children that won't need to be moved.

    5. Identify which nodes need to be removed
    6. Identify which nodes will need to be diffed

    7. Going along each item in the new list, create it and insert it before the next closest item in the LIS.
        - if the item already existed, just move it to the right place.

    8. Finally, generate instructions to remove any old children.
    9. Generate instructions to finally diff children that are the same between both
    */

    // 0. Debug sanity checks
    // Should have already diffed the shared-key prefixes and suffixes.
    debug_assert_ne!(new.first().map(VNode::key), old.first().map(VNode::key));
    debug_assert_ne!(new.last().map(VNode::key), old.last().map(VNode::key));

    // 1. Map the old keys into a numerical ordering based on indices.
    // 2. Create a map of old key to its index
    // IE if the keys were A B C, then we would have (A, 1) (B, 2) (C, 3).
    let old_key_to_old_index = old
        .iter()
        .enumerate()
        .map(|(i, o)| (o.key().unwrap(), i))
        .collect::<FxHashMap<_, _>>();

    let mut shared_keys = FxHashSet::default();

    // 3. Map each new key to the old key, carrying over the old index.
    let new_index_to_old_index = new
        .iter()
        .map(|node| {
            let key = node.key().unwrap();
            if let Some(&index) = old_key_to_old_index.get(&key) {
                shared_keys.insert(key);
                index
            } else {
                u32::MAX as usize
            }
        })
        .collect::<Vec<_>>();

    // If none of the old keys are reused by the new children, then we remove all the remaining old children and
    // create the new children afresh.
    if shared_keys.is_empty() {
        if let Some(first_old) = old.get(0) {
            &old[1..].destroy(registry);
            new.create(registry);
            let nodes_created = registry.take_created();
            self.replace_inner(first_old, nodes_created);
        } else {
            // I think this is wrong - why are we appending?
            // only valid of the if there are no trailing elements
            create_and_append_children(new, registry);
        }
        return;
    }

    // remove any old children that are not shared
    // todo: make this an iterator
    for child in old {
        let key = child.key().unwrap();
        if !shared_keys.contains(&key) {
            self.remove_nodes([child], true);
        }
    }

    // 4. Compute the LIS of this list
    let mut lis_sequence = Vec::default();
    lis_sequence.reserve(new_index_to_old_index.len());

    let mut predecessors = vec![0; new_index_to_old_index.len()];
    let mut starts = vec![0; new_index_to_old_index.len()];

    longest_increasing_subsequence::lis_with(
        &new_index_to_old_index,
        &mut lis_sequence,
        |a, b| a < b,
        &mut predecessors,
        &mut starts,
    );

    // the lis comes out backwards, I think. can't quite tell.
    lis_sequence.sort_unstable();

    // if a new node gets u32 max and is at the end, then it might be part of our LIS (because u32 max is a valid LIS)
    if lis_sequence.last().map(|f| new_index_to_old_index[*f]) == Some(u32::MAX as usize) {
        lis_sequence.pop();
    }

    for idx in &lis_sequence {
        self.diff_node(&old[new_index_to_old_index[*idx]], &new[*idx]);
    }

    let mut nodes_created = 0;

    // add mount instruction for the first items not covered by the lis
    let last = *lis_sequence.last().unwrap();
    if last < (new.len() - 1) {
        for (idx, new_node) in new[(last + 1)..].iter().enumerate() {
            let new_idx = idx + last + 1;
            let old_index = new_index_to_old_index[new_idx];
            if old_index == u32::MAX as usize {
                nodes_created += self.create_node(new_node);
            } else {
                self.diff_node(&old[old_index], new_node);
                nodes_created += self.push_all_real_nodes(new_node);
            }
        }

        self.mutations.insert_after(
            self.find_last_element(&new[last]).unwrap(),
            nodes_created as u32,
        );
        nodes_created = 0;
    }

    // for each spacing, generate a mount instruction
    let mut lis_iter = lis_sequence.iter().rev();
    let mut last = *lis_iter.next().unwrap();
    for next in lis_iter {
        if last - next > 1 {
            for (idx, new_node) in new[(next + 1)..last].iter().enumerate() {
                let new_idx = idx + next + 1;
                let old_index = new_index_to_old_index[new_idx];
                if old_index == u32::MAX as usize {
                    nodes_created += self.create_node(new_node);
                } else {
                    self.diff_node(&old[old_index], new_node);
                    nodes_created += self.push_all_real_nodes(new_node);
                }
            }

            self.mutations.insert_before(
                self.find_first_element(&new[last]).unwrap(),
                nodes_created as u32,
            );

            nodes_created = 0;
        }
        last = *next;
    }

    // add mount instruction for the last items not covered by the lis
    let first_lis = *lis_sequence.first().unwrap();
    if first_lis > 0 {
        for (idx, new_node) in new[..first_lis].iter().enumerate() {
            let old_index = new_index_to_old_index[idx];
            if old_index == u32::MAX as usize {
                nodes_created += self.create_node(new_node);
            } else {
                new_node.diff(&old[old_index], registry);
                nodes_created += self.push_all_real_nodes(new_node);
            }
        }

        registry.mutations.insert_before(
            find_first_element(&new[first_lis], registry).unwrap(),
            nodes_created as u32,
        );
    }
}

fn find_last_element<'b>(
    vnode: &'b VNode<'b>,
    registry: &mut VDomRegestry<'b>,
) -> Option<ElementId> {
    let mut search_node = Some(vnode);
    loop {
        match &search_node.take().unwrap() {
            VNode::Text(t) => break t.id.get(),
            VNode::Element(t) => break t.id.get(),
            VNode::Placeholder(t) => break t.id.get(),
            VNode::Fragment(frag) => search_node = frag.children.last(),
            VNode::Component(el) => {
                let scope_id = el.scope.get().unwrap();
                search_node = Some(registry.scopes.root_node(scope_id));
            }
        }
    }
}

fn find_first_element<'b>(
    vnode: &'b VNode<'b>,
    registry: &mut VDomRegestry<'b>,
) -> Option<ElementId> {
    let mut search_node = Some(vnode);
    loop {
        match &search_node.take().expect("search node to have an ID") {
            VNode::Text(t) => break t.id.get(),
            VNode::Element(t) => break t.id.get(),
            VNode::Placeholder(t) => break t.id.get(),
            VNode::Fragment(frag) => search_node = Some(&frag.children[0]),
            VNode::Component(el) => {
                let scope = el.scope.get().expect("element to have a scope assigned");
                search_node = Some(registry.scopes.root_node(scope));
            }
        }
    }
}

fn create_and_insert_after<'b>(
    nodes: &'b [VNode<'b>],
    after: &'b VNode<'b>,
    registry: &mut VDomRegestry<'b>,
) {
    nodes.create(registry);
    let created = registry.take_created();
    let last = find_last_element(after, registry).unwrap();
    registry.mutations.insert_after(last, created as u32);
}

fn create_and_insert_before<'b>(
    nodes: &'b [VNode<'b>],
    before: &'b VNode<'b>,
    registry: &mut VDomRegestry<'b>,
) {
    nodes.create(registry);
    let created = registry.nodes_to_place;
    let first = find_first_element(before, registry).unwrap();
    registry.mutations.insert_before(first, created as u32);
}

fn create_and_append_children<'b>(nodes: &'b [VNode<'b>], registry: &mut VDomRegestry<'b>) {
    nodes.create(registry);
    let created = registry.take_created();
    registry.mutations.append_children(created as u32);
}
