#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    dioxus_desktop::launch(root);

    #[cfg(target_arch = "wasm32")]
    dioxus_web::launch(root);
}

fn root(cx: Scope) -> Element {
    render! {
        Router {
            config: RouterConfiguration {
                history: {
                    #[cfg(not(target_arch = "wasm32"))]
                    let history = Box::<MemoryHistory::<Route>>::default();
                    #[cfg(target_arch = "wasm32")]
                    let history = Box::<WebHistory::<Route>>::default();
                    history
                },
                ..Default::default()
            }
        }
    }
}

// #[inline_props]
// fn UserFrame(cx: Scope, user_id: usize) -> Element {
//     render! {
//         pre {
//             "UserFrame{{\n\tuser_id:{user_id}\n}}"
//         }
//         div {
//             background_color: "rgba(0,0,0,50%)",
//             "children:"
//             Outlet {}
//         }
//     }
// }

// #[inline_props]
// fn Route1(cx: Scope, user_id: usize, dynamic: usize, extra: String) -> Element {
//     render! {
//         pre {
//             "Route1{{\n\tuser_id:{user_id},\n\tdynamic:{dynamic},\n\textra:{extra}\n}}"
//         }
//         Link {
//             target: Route::Route1 { user_id: *user_id, dynamic: *dynamic, extra: extra.clone() + "." },
//             "Route1 with extra+\".\""
//         }
//         p { "Footer" }
//         Link {
//             target: Route::Route3 { dynamic: String::new() },
//             "Home"
//         }
//     }
// }

// #[inline_props]
// fn Route2(cx: Scope, user_id: usize) -> Element {
//     render! {
//         pre {
//             "Route2{{\n\tuser_id:{user_id}\n}}"
//         }
//         (0..*user_id).map(|i| rsx!{ p { "{i}" } }),
//         p { "Footer" }
//         Link {
//             target: Route::Route3 { dynamic: String::new() },
//             "Home"
//         }
//     }
// }

fn Route3(cx: Scope<Route3Props>) -> Element {
    let dynamic = cx.props.dynamic;
    let router = use_router(cx);
    let router_route = router.current();
    let current_route = use_ref(cx, String::new);
    let parsed = Route::from_str(&current_route.read());

    let site_map = Route::SITE_MAP
        .iter()
        .flat_map(|seg| seg.flatten().into_iter())
        .collect::<Vec<_>>();

    render! {
        input {
            oninput: move |evt| {
                *current_route.write() = evt.value.clone();
            },
            value: "{current_route.read()}"
        }
        "dynamic: {dynamic}"
        // Link {
        //     target: Route::Route2 { user_id: 8888 },
        //     "hello world link"
        // }
        p { "Site Map" }
        pre { "{site_map:#?}" }
        p { "Dynamic link" }
        if let Ok(route) = parsed {
            if route != router_route {
                render! {
                    Link {
                        target: route.clone(),
                        "{route}"
                    }
                }
            }
            else {
                None
            }
        }
        else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Route3Props {
    dynamic: usize,
}

routes! {
    #[derive(Debug, Clone, PartialEq)]
    Route,
    // All nests that have dynamic segments must have a name used to generate a struct
    // route(User, "user" / user_id: usize ) {
    //     route(Product, "product" / product_id: usize / dynamic: usize ) {
    //         // Render creates a new route (that will be included in the enum) and is rendered with the given component
    //         // The component uses the struct of the parent route as a prop (in this case, Product)
    //         render(Other)
    //     }

    //     // You can nest routes inside a layout to wrap them in a component that accepts the struct of the parent route as a prop (in this case, User)
    //     layout(UserFrame) {
    //         route(Route1Props, "hello_world" / dynamic: usize ) {
    //             // (Accepts Route1Props as a prop)
    //             render(Route1)
    //         }

    //         // You can opt out of the layout by using !layout
    //         !layout(UserFrame) {
    //             route(Route2Props, "hello_world" / dynamic: usize ) {
    //                 // (Accepts Route2Props as a prop)
    //                 render(Route2)
    //             }
    //         }
    //     }
    // }

    route(Route3Props, "hello_world" / dynamic: usize ) {
        // (Accepts Route3Props as a prop)
        render(Route3)
    }

    // route(RedirectData, dynamic: usize / extra: String) {
    //     // Redirects accept a function that receives the struct of the parent route and returns the new route
    //     redirect(|data: RedirectData| { Route::Route1 { user_id: 0, dynamic: data.dynamic, extra: data.extra.to_string()} })
    // }
}
