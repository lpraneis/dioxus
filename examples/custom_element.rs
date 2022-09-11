//! This example shows to wrap a webcomponent / custom element with a component.
//!
//! Oftentimes, a third party library will provide a webcomponent that you want
//! to use in your application. This example shows how to create that custom element
//! directly with the raw_element method on NodeFactory.

use dioxus::prelude::*;

fn main() {
    let mut dom = VirtualDom::new(app);
    let _ = dom.rebuild();

    let output = dioxus_ssr::render_vdom(&dom);

    println!("{}", output);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        "my-element" {
            "client-id": "abc123",
            "name": "bob",
            "age": "47",
        }
    })
}
