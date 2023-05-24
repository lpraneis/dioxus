use dioxus::prelude::{dioxus_hot_reload::Config, *};

fn main() {
    hot_reload_init!(Config::new().with_rebuild_command(""));
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(div{
        p{
            "p1"
        }
        // Adding this extra para does nothing
        p{
            "p2"
        }
    }))
}
