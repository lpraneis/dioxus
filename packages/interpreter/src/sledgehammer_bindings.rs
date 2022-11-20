use dioxus_core::{ElementId, MutationStore, ScopeId};
use dioxus_html::event_bubbles;
use sledgehammer_bindgen::bindgen;
use std::convert::TryInto;
use ux::*;

#[bindgen]
extern "C" {
    fn initialize() {
        r#"
class ListenerMap {
    constructor(root, handler) {
        // bubbling events can listen at the root element
        this.global = {};
        // non bubbling events listen at the element the listener was created at
        this.local = {};
        this.root = root;
        this.handler = handler;
    }
    
    create(event_name, element, bubbles) {
        if (bubbles) {
            if (this.global[event_name] === undefined) {
                this.global[event_name] = {};
                this.global[event_name].active = 1;
                this.global[event_name].callback = this.handler;
                this.root.addEventListener(event_name, this.handler);
            } else {
                this.global[event_name].active++;
            }
        }
        else {
            const id = element.getAttribute("data-dioxus-id");
            if (!this.local[id]) {
                this.local[id] = {};
            }
            this.local[id][event_name] = handler;
            element.addEventListener(event_name, this.handler);
        }
    }
    
    remove(element, event_name, bubbles) {
        if (bubbles) {
            this.global[event_name].active--;
            if (this.global[event_name].active === 0) {
                this.root.removeEventListener(event_name, this.global[event_name].callback);
                delete this.global[event_name];
            }
        }
        else {
            const id = element.getAttribute("data-dioxus-id");
            delete this.local[id][event_name];
            if (this.local[id].length === 0) {
                delete this.local[id];
            }
            this.element.removeEventListener(event_name, this.handler);
        }
    }
    
    removeAllNonBubbling(element) {
        const id = element.getAttribute("data-dioxus-id");
        delete this.local[id];
    }
}
let listeners, nodes, stack, templates;
{
    let root = window.document.getElementById("main");
    console.log("interpreter created", root);
    listeners = new ListenerMap(root, ()=>console.log("todo"));
    nodes = [root];
    stack = [root];
}
templates = {};
function LoadChild(path) {
    // iterate through each number and get that child
    let node = stack[stack.length - 1];
    let len = path.length;
    let child_num;
    for (let i = 0; i < len; i++) {
        child_num = path[i];
        // iterating through next sibling is faster than indexing childNodes
        node = node.firstChild;
        for (let j = 0; j < child_num; j++) {
            node = node.nextSibling;
        }
    }

    return node;
}
"#
    }
    fn append_children(many: u8) {
        "{
            let parent = stack[stack.length - (1 + many)];
            let to_add = stack.splice(stack.length - many);
            for (let i = 0; i < many; i++) {
                parent.appendChild(to_add[i]);
            }
        }"
    }
    fn replace(id: u24, n: u8) {
        "{
            let root = nodes[id];
            let els = stack.splice(stack.length - n);
            if (is_element_node(root.nodeType)) {
                listeners.removeAllNonBubbling(root);
            }
            root.replaceWith(...els);
        }"
    }
    fn insert_after(id: u24, n: u8) {
        "{
            nodes[id].after(...stack.splice(stack.length - n));
        }"
    }
    fn insert_before(id: u24, n: u8) {
        "{
            nodes[id].before(...stack.splice(stack.length - n));
        }"
    }
    fn remove(id: u24) {
        "{
            let node = nodes[id];
            if (node !== undefined) {
                if (is_element_node(node)) {
                    listeners.removeAllNonBubbling(node);
                }
                node.remove();
            }
        }"
    }
    fn create_raw_text(text: &str) {
        "{
            stack.push(document.createTextNode(text));
        }"
    }
    fn create_text_node(text: &str, id: u24) {
        "{
            const node = document.createTextNode(text);
            nodes[id] = node;
            stack.push(node);
        }"
    }
    fn create_element(tag: &str<u8>, id: u24) {
        "{
            const el = document.createElement(tag);
            nodes[id] = el;
            stack.push(el);
        }"
    }
    fn create_element_ns(tag: &str<u8>, ns: &str<u8>, id: u24) {
        r#"{
            console.log("creating element", tag, id, ns);
            let el = document.createElementNS(ns, tag);
            stack.push(el);
            nodes[id] = el;
        }"#
    }
    fn create_placeholder(id: u24) {
        r#"{
            let el = document.createElement("pre");
            el.hidden = true;
            stack.push(el);
            nodes[id] = el;
        }"#
    }
    fn new_event_listener(id: u24, event_name: &str<u8>, bubbles: u8) {
        r#"{
            const element = nodes[id];
            element.setAttribute("data-dioxus-id", id);
            listeners.create(event_name, element, bubbles);
        }"#
    }
    fn remove_event_listener(id: u24, event_name: &str<u8>, bubbles: u8) {
        r#"{
            const element = nodes[id];
            element.removeAttribute(`data-dioxus-id`);
            listeners.remove(element, event_name, bubbles);
        }"#
    }
    fn set_text(id: u24, text: &str<u8>) {
        "{
            nodes[id].textContent = text;
        }"
    }
    fn set_attribute(id: u24, field: &str<u8>, value: &str<u16>) {
        r#"{
            const node = nodes[id];
            switch (field) {
                case "value":
                if (value !== node.value) {
                    node.value = value;
                }
                break;
                case "checked":
                node.checked = value === "true";
                break;
                case "selected":
                node.selected = value === "true";
                break;
                case "dangerous_inner_html":
                node.innerHTML = value;
                break;
                default:
                // https://github.com/facebook/react/blob/8b88ac2592c5f555f315f9440cbb665dd1e7457a/packages/react-dom/src/shared/DOMProperty.js#L352-L364
                if (value === "false" && bool_attrs.hasOwnProperty(field)) {
                    node.removeAttribute(field);
                } else {
                    node.setAttribute(field, value);
                }
            }
        }"#
    }
    fn set_attribute_ns(id: u24, field: &str<u8>, ns: &str<u8>, value: &str<u16>) {
        r#"{
            const node = nodes[id];
            if (ns === "style") {
                // ????? why do we need to do this
                if (node.style === undefined) {
                    node.style = {};
                }
                node.style[field] = value;
            } else if (ns != null && ns != undefined) {
                node.setAttributeNS(ns, field, value);
            }
        }"#
    }
    fn remove_attribute(id: u24, field: &str<u8>, ns: &str<u8>) {
        r#"{
            const node = nodes[id];
            switch (ns){
                case "style":
                    node.style.removeProperty(field);
                    break;
                case null:
                case undefined:
                    node.removeAttributeNS(ns, field);
                    break;
                default:
                    switch (field){
                        case "value":
                            node.value = "";
                            break;
                        case "checked":
                            node.checked = false;
                            break;
                        case "selected":
                            node.selected = false;
                            break;
                        case "dangerous_inner_html":
                            node.innerHTML = "";
                            break;
                        default:
                            node.removeAttribute(field);
                    }
            }
        }"#
    }
    fn assign_id(path: &[u8<u8>], id: u24) {
        "{
            nodes[id] = LoadChild(path);
        }"
    }
    fn hydrate_text(path: &[u8<u8>], value: &str<u16>, id: u24) {
        "{
            const node = LoadChild(path);
            node.textContent = value;
            nodes[id] = node;
        }"
    }
    fn replace_placeholder(path: &[u8<u8>], id: u24) {
        "{
            LoadChild(path).replaceWith(...stack.splice(stack.length - n));
        }"
    }
    fn load_template(name: &str<u8>, index: u8) {
        r#"{
            console.log("loading template", name, index);
            stack.push(templates[name][index].cloneNode(true));
        }"#
    }
    fn save_template(name: &str<u8>, n: u8) {
        "{
            templates[name] = stack.splice(stack.length - n);
        }"
    }
}

#[derive(Default)]
struct ByteMutations {
    channel: Channel,
    opertaions: usize,
}

impl<'a> MutationStore<'a> for ByteMutations {
    fn append(&mut self, other: Self) {
        self.channel.append(other.channel);
        self.opertaions += other.opertaions;
    }
    fn len(&self) -> usize {
        self.opertaions
    }
    fn set_attribute(&mut self, attr: &'a str, ns: Option<&'a str>, value: &'a str, id: ElementId) {
        match ns {
            Some(ns) => self
                .channel
                .set_attribute_ns(id.0.try_into().unwrap(), attr, ns, value),
            None => self
                .channel
                .set_attribute(id.0.try_into().unwrap(), attr, value),
        }
        self.opertaions += 1;
    }
    fn set_bool_attribute(
        &mut self,
        attr: &'a str,
        ns: Option<&'a str>,
        value: bool,
        id: ElementId,
    ) {
        match ns {
            Some(ns) => self.channel.set_attribute_ns(
                id.0.try_into().unwrap(),
                attr,
                ns,
                &value.to_string(),
            ),
            None => self
                .channel
                .set_attribute(id.0.try_into().unwrap(), attr, &value.to_string()),
        }
        self.opertaions += 1;
    }
    fn load_template(&mut self, id: &'static str, index: usize) {
        self.channel.load_template(id, index as u8);
        self.opertaions += 1;
    }
    fn save_template(&mut self, id: &'static str, index: usize) {
        self.channel.save_template(id, index as u8);
        self.opertaions += 1;
    }
    fn hydrate_text(&mut self, path: &'static [u8], value: &'a str, id: ElementId) {
        self.channel
            .hydrate_text(path, value, id.0.try_into().unwrap());
        self.opertaions += 1;
    }
    fn set_text(&mut self, value: &'a str, id: ElementId) {
        self.channel.set_text(id.0.try_into().unwrap(), value);
        self.opertaions += 1;
    }
    fn replace_placeholder(&mut self, id: usize, path: &'static [u8]) {
        self.channel
            .replace_placeholder(path, (id as u32).try_into().unwrap());
        self.opertaions += 1;
    }
    fn assign_id(&mut self, path: &'static [u8], id: ElementId) {
        self.channel.assign_id(path, id.0.try_into().unwrap());
        self.opertaions += 1;
    }
    fn replace(&mut self, id: ElementId, m: usize) {
        self.channel.replace(id.0.try_into().unwrap(), m as u8);
        self.opertaions += 1;
    }
    fn create_element(&mut self, tag: &'a str, ns: Option<&'a str>, id: ElementId) {
        match ns {
            Some(ns) => self
                .channel
                .create_element_ns(ns, tag, id.0.try_into().unwrap()),
            None => self.channel.create_element(tag, id.0.try_into().unwrap()),
        }
        self.opertaions += 1;
    }
    fn set_inner_text(&mut self, text: &'a str) {
        self.channel.create_raw_text(text);
        self.opertaions += 1;
    }
    fn create_text(&mut self, id: ElementId, text: &'a str) {
        self.channel
            .create_text_node(text, id.0.try_into().unwrap());
        self.opertaions += 1;
    }
    fn create_static_text(&mut self, text: &'a str) {
        self.channel.create_raw_text(text);
        self.opertaions += 1;
    }
    fn create_placeholder(&mut self, id: ElementId) {
        self.channel.create_placeholder(id.0.try_into().unwrap());
        self.opertaions += 1;
    }
    fn new_event_listener(&mut self, event: &'a str, _: ScopeId, id: ElementId) {
        self.channel.new_event_listener(
            id.0.try_into().unwrap(),
            event,
            event_bubbles(event) as u8,
        );
        self.opertaions += 1;
    }
    fn remove_event_listener(&mut self, id: ElementId, event: &'a str) {
        self.channel.remove_event_listener(
            id.0.try_into().unwrap(),
            event,
            event_bubbles(event) as u8,
        );
        self.opertaions += 1;
    }
    fn append_children(&mut self, m: usize) {
        self.channel.append_children(m as u8);
    }
}
