//! Run with:
//!
//! ```sh
//! dioxus build --features web
//! cargo run --features ssr --no-default-features
//! ```

#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_fullstack::prelude::*;
use dioxus_router::*;
use serde::{Deserialize, Serialize};

fn main() {
    #[cfg(feature = "web")]
    {
        wasm_logger::init(wasm_logger::Config::default());

        dioxus_web::launch_with_props(
            App,
            AppProps { route: None },
            dioxus_web::Config::new().hydrate(true),
        );
    }
    #[cfg(feature = "ssr")]
    {
        // Start hot reloading
        hot_reload_init!(dioxus_hot_reload::Config::new().with_rebuild_callback(|| {
            execute::shell("dioxus build --features web")
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
            execute::shell("cargo run --features ssr --no-default-features")
                .spawn()
                .unwrap();
            true
        }));

        use axum::extract::State;

        SubOneServer::register().unwrap();
        AddOneServer::register().unwrap();
        GetCountServer::register().unwrap();

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));

                axum::Server::bind(&addr)
                    .serve(
                        axum::Router::new()
                            // Serve the dist/assets folder with the javascript and WASM files created by the CLI
                            .serve_static_assets("./dist")
                            // Register server functions
                            .register_server_fns("")
                            // Connect to the hot reload server
                            .connect_hot_reload()
                            // If the path is unknown, render the application
                            .fallback(
                                move |uri: http::uri::Uri, State(ssr_state): State<SSRState>| {
                                    let rendered = ssr_state.render(
                                        &ServeConfigBuilder::new(
                                            App,
                                            AppProps {
                                                route: Some(format!("http://{addr}{uri}")),
                                            },
                                        )
                                        .build(),
                                    );
                                    async move { axum::body::Full::from(rendered) }
                                },
                            )
                            .with_state(SSRState::default())
                            .into_make_service(),
                    )
                    .await
                    .unwrap();
            });
    }
}

#[derive(Clone, Debug, Props, PartialEq, Serialize, Deserialize)]
struct AppProps {
    route: Option<String>,
}

fn App(cx: Scope<AppProps>) -> Element {
    cx.render(rsx! {
        Router {
            initial_url: cx.props.route.clone(),

            Route { to: "/blog",
                Link {
                    to: "/",
                    "Go to counter"
                }
                table {
                    tbody {
                        for _ in 0..100 {
                            tr {
                                for _ in 0..100 {
                                    td { "hello world!" }
                                }
                            }
                        }
                    }
                }
            },
            // Fallback
            Route { to: "",
                Counter {}
            },
        }
    })
}

fn Counter(cx: Scope) -> Element {
    let count = {
        #[cfg(feature = "ssr")]
        {
            count()
        }
        #[cfg(not(feature = "ssr"))]
        {
            use_future!(cx, |()| async { get_count().await.unwrap() })
        }
    };

    cx.render(rsx! {
        {
            #[cfg(not(feature = "ssr"))]
            let count = count.value()
                .cloned()
                .unwrap_or_default();
            rsx! {
                h1 { "High-Five counter: {count}" }
            }
        }
        Form::<AddOneServer> {
            onsubmit: move |_| {
                #[cfg(not(feature = "ssr"))]
                count.restart();
            },
            button {
                r#type: "submit",
                "Up high!"
            }
        }
        Form::<SubOneServer> {
            onsubmit: move |_| {
                #[cfg(not(feature = "ssr"))]
                count.restart();
            },
            button {
                r#type: "submit",
                "Down low!"
            }
        }
    })
}

#[cfg(feature = "ssr")]
use server::*;
#[cfg(feature = "ssr")]
mod server {
    use std::sync::atomic::{AtomicIsize, Ordering};

    static COUNT: AtomicIsize = AtomicIsize::new(0);

    pub(crate) fn add() {
        COUNT.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) fn sub() {
        COUNT.fetch_sub(1, Ordering::SeqCst);
    }

    pub(crate) fn count() -> isize {
        COUNT.load(Ordering::SeqCst)
    }
}

#[server(AddOneServer, "", "Url")]
async fn add_one() -> Result<(), ServerFnError> {
    add();

    Ok(())
}

#[server(SubOneServer, "", "Url")]
async fn sub_one() -> Result<(), ServerFnError> {
    sub();

    Ok(())
}

#[server(GetCountServer)]
async fn get_count() -> Result<isize, ServerFnError> {
    Ok(count())
}
