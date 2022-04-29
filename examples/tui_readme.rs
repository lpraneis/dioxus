use dioxus::prelude::*;

fn main() {
    dioxus::tui::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            // justify_content: "center",
            // align_items: "center",

            p{
                // flex_direction: "column",
                "Hello world! Hello world! Hello world! Hello world! Hello world! Hello world!"
                // "Hello world! Hello world! Hello world! Hello world! Hello world! Hello world!"
            }
            // p{"Hello world! Hello world! Hello world! Hello world! Hello world! Hello world!"}
        }
    })
}
