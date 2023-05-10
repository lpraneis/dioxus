//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Component {
            onclick: |evt| println!("clicked: {:?}", evt)
        }
    })
}

#[derive(Props)]
struct ComponentProps<'a> {
    onclick: Option<EventHandler<'a, MouseEvent>>,
}

fn Component<'a>(cx: Scope<'a, ComponentProps<'a>>) -> Element<'a> {
    let has_listener = use_ref(cx, || false);
    let currently_has_listener = dbg!(*has_listener.read());
    let onclick = cx.props.onclick.as_ref().filter(|_| currently_has_listener);

    let txt = if currently_has_listener {
        "Remove listener"
    } else {
        "Add listener"
    };
    render! {
        button {
            onclick: onclick,
            "button"
        }
        button {
            // This type cannot be infered anymore, so we have to specify it.
            onkeydown: |evt: KeyboardEvent| {
                println!("keydown: {:?}", evt);
                evt.data.modifiers();
            },
            onclick: |evt| {
                println!("clicked: {:?}", evt);
                let mut has_listener = has_listener.write();
                *has_listener = !*has_listener;
            },
            "{txt}"
        }
    }
}
