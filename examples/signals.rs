use dioxus::prelude::*;
use dioxus_signals::{use_init_signal_rt, use_signal, Signal};
use std::time::Duration;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    println!("app");
    use_init_signal_rt(cx);

    let mut count = use_signal(cx, || 0);
    let child_count = use_signal(cx, || 0);
    cx.provide_context(child_count);

    use_future!(cx, || async move {
        loop {
            count += 1;
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
    });

    cx.render(rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }

        if count() > 5 {
            rsx!{ h2 { "High five!" } }
        }

        child_comp {}
    })
}

fn child_comp(cx: Scope) -> Element {
    println!("child_comp");
    let mut count: Signal<i32> = *use_context(cx).unwrap();

    cx.render(rsx! {
        h1 { "Child counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    })
}
