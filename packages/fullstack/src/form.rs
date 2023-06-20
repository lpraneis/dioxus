#![allow(non_snake_case)]

use std::rc::Rc;

use dioxus::prelude::*;
use serde::de::DeserializeOwned;

use crate::{server_context::DioxusServerContext, server_fn::ServerFn};

/// A form data event
#[derive(Props)]
pub struct FormProps<'a, A: Action> {
    #[props(default = "application/x-www-form-urlencoded")]
    encoding: &'a str,
    #[props(default = "POST")]
    method: &'a str,
    onsubmit: Option<EventHandler<'a, ()>>,
    onchange: Option<EventHandler<'a, Rc<FormData>>>,
    #[props(default = std::marker::PhantomData)]
    phantom: std::marker::PhantomData<A>,
    children: Element<'a>,
}

/// A progressively enhanced form that can be used to submit data to the server.
pub fn Form<'a, A: Action>(cx: Scope<'a, FormProps<'a, A>>) -> Element<'a> {
    render! {
        form {
            action: A::submit_url(),
            prevent_default: "onsubmit",
            enctype: cx.props.encoding,
            method: cx.props.method,
            onsubmit: |evt| {
                A::onsubmit(cx, evt.inner().clone());
                if let Some(onsubmit) = &cx.props.onsubmit {
                    onsubmit.call(())
                }
            },
            onchange: |evt| {
                if let Some(onchange) = &cx.props.onchange {
                    onchange.call(evt.inner().clone())
                }
            },
            &cx.props.children
        }
    }
}

/// A form action
pub trait Action {
    /// The url to submit the form to in SSR mode
    fn submit_url() -> &'static str;
    /// The onsubmit event handler in client mode
    fn onsubmit(cx: &ScopeState, evt: Rc<FormData>);
}

impl<F: ServerFn + DeserializeOwned + Clone> Action for F {
    fn submit_url() -> &'static str {
        <Self as server_fn::ServerFn<DioxusServerContext>>::url()
    }

    #[allow(unused)]
    fn onsubmit(cx: &ScopeState, evt: Rc<FormData>) {
        #[cfg(not(feature = "ssr"))]
        {
            let mut url_encoded = String::new();
            for (k, v) in &evt.values {
                url_encoded.push_str(&format!("{}={}&", k, v[0]));
            }
            let url_encoded = url_encoded.trim_end_matches('&').to_string();
            log::info!("Submitting form: {}", url_encoded);
            #[cfg(feature = "router")]
            let router = cx.consume_context::<dioxus_router::RouterContext>();
            cx.spawn(async move {
                let client = reqwest::Client::default();
                let response = client
                    .post(format!("http://127.0.0.1:8080/{}", Self::submit_url()))
                    .body(url_encoded)
                    .send()
                    .await;

                match response {
                    Ok(res) => {
                        #[cfg(feature = "router")]
                        if let Some(router) = router {
                            log::info!("response: {:#?}, {:?}", res, res.url());
                            let new_url = res.url();

                            log::info!("Redirecting to: {}", new_url);
                            let current_url = &router.current_location().url;
                            router.navigate_to(new_url.as_str());
                        }
                        res.bytes().await;
                    }
                    Err(err) => {
                        log::error!("Failed to submit form: {}", err);
                    }
                }
            });
        }
    }
}
